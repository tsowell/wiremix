mod deserialize;
mod device;
mod device_status;
mod event_sender;
mod link;
mod metadata;
mod node;
mod proxy_registry;
mod stream;
mod stream_registry;

use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::thread;

use nix::sys::eventfd::{EfdFlags, EventFd};
use std::os::fd::AsRawFd;

use pipewire::{
    main_loop::MainLoop,
    properties::properties,
    proxy::{Listener, ProxyT},
    types::ObjectType,
};

use crate::event::{Event, MonitorEvent};
use crate::monitor::{
    device_status::DeviceStatusTracker, event_sender::EventSender,
    proxy_registry::ProxyRegistry, stream_registry::StreamRegistry,
};
use crate::object::ObjectId;

type ProxyInfo = (Box<Rc<dyn ProxyT>>, Box<dyn Listener>);

pub fn spawn(
    remote: Option<String>,
    tx: Arc<mpsc::Sender<Event>>,
    is_capture_enabled: bool,
) -> Result<MonitorHandle> {
    let shutdown_fd =
        Arc::new(EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?);

    let handle = thread::spawn({
        let shutdown_fd = Arc::clone(&shutdown_fd);
        move || {
            let _ = run(remote, tx, shutdown_fd, is_capture_enabled);
        }
    });

    Ok(MonitorHandle {
        fd: Some(shutdown_fd),
        handle: Some(handle),
    })
}

fn run(
    remote: Option<String>,
    tx: Arc<mpsc::Sender<Event>>,
    shutdown_fd: Arc<EventFd>,
    is_capture_enabled: bool,
) -> Result<()> {
    pipewire::init();

    let _guard = scopeguard::guard((), |_| unsafe {
        pipewire::deinit();
    });

    let main_loop = MainLoop::new(None)?;
    let sender = Rc::new(EventSender::new(tx, main_loop.downgrade()));

    let err_sender = Rc::clone(&sender);
    monitor_pipewire(
        remote,
        main_loop,
        sender,
        shutdown_fd,
        is_capture_enabled,
    )
    .unwrap_or_else(move |e| {
        err_sender.send_error(e.to_string());
    });

    Ok(())
}

pub struct MonitorHandle {
    fd: Option<Arc<EventFd>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for MonitorHandle {
    fn drop(&mut self) {
        if let Some(fd) = self.fd.take() {
            let _ = fd.arm();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn monitor_pipewire(
    remote: Option<String>,
    main_loop: MainLoop,
    sender: Rc<EventSender>,
    shutdown_fd: Arc<EventFd>,
    is_capture_enabled: bool,
) -> Result<()> {
    let context = pipewire::context::Context::new(&main_loop)?;
    let props = remote.map(|remote| {
        properties! {
            *pipewire::keys::REMOTE_NAME => remote
        }
    });
    let core = context.connect(props)?;

    let fd = shutdown_fd.as_raw_fd();
    let _shutdown_watch =
        main_loop
            .loop_()
            .add_io(fd, libspa::support::system::IoFlags::IN, {
                let main_loop_weak = main_loop.downgrade();
                move |_status| {
                    if let Some(main_loop) = main_loop_weak.upgrade() {
                        main_loop.quit();
                    }
                }
            });

    let _core_listener = core
        .add_listener_local()
        .error({
            let main_loop_weak = main_loop.downgrade();
            let sender_weak = Rc::downgrade(&sender);
            move |_id, _seq, _res, message| {
                if let Some(main_loop) = main_loop_weak.upgrade() {
                    main_loop.quit();
                }
                if let Some(sender) = sender_weak.upgrade() {
                    sender.send_error(message.to_string());
                };
            }
        })
        .register();

    let registry = Rc::new(core.get_registry()?);
    let registry_weak = Rc::downgrade(&registry);

    // Proxies and their listeners need to stay alive so store them here
    let proxies = Rc::new(RefCell::new(ProxyRegistry::new()));
    let streams = Rc::new(RefCell::new(StreamRegistry::new()));

    let statuses = Rc::new(RefCell::new(DeviceStatusTracker::new()));

    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let obj_id = ObjectId::from(obj);

            let Some(registry) = registry_weak.upgrade() else {
                return;
            };
            let (p, s) = match obj.type_ {
                ObjectType::Node => {
                    let p = node::monitor_node(&registry, obj, &sender);
                    let s = if is_capture_enabled {
                        stream::capture_node(&core, obj, &sender, obj_id)
                    } else {
                        None
                    };

                    (p, s)
                }
                ObjectType::Device => (
                    device::monitor_device(&registry, obj, &sender, &statuses),
                    None,
                ),
                ObjectType::Link => {
                    (link::monitor_link(&registry, obj, &sender), None)
                }
                ObjectType::Metadata => {
                    (metadata::monitor_metadata(&registry, obj, &sender), None)
                }
                _ => (None, None),
            };

            let Some((proxy_spe, listener_spe)) = p else {
                return;
            };

            let proxy = proxy_spe.upcast_ref();
            let proxy_id = proxy.id();
            // Use a weak ref to prevent references cycle between Proxy and proxies:
            // - ref on proxies in the closure, bound to the Proxy lifetime
            // - proxies owning a ref on Proxy as well
            let proxies_weak = Rc::downgrade(&proxies);

            let stream_info = s.as_ref().map(|(stream_spe, _)| {
                (Rc::downgrade(&streams), Rc::downgrade(stream_spe))
            });
            let sender_weak = Rc::downgrade(&sender);
            let listener = proxy
                .add_listener_local()
                .removed(move || {
                    let Some(sender) = sender_weak.upgrade() else {
                        return;
                    };
                    let Some(proxies) = proxies_weak.upgrade() else {
                        return;
                    };

                    proxies.borrow_mut().remove(proxy_id);

                    sender.send(MonitorEvent::Removed(obj_id));

                    let Some((ref streams_weak, ref stream_spe_weak)) =
                        stream_info
                    else {
                        return;
                    };
                    let Some(streams) = streams_weak.upgrade() else {
                        return;
                    };
                    let Some(stream_spe) = stream_spe_weak.upgrade() else {
                        return;
                    };

                    let _ = stream_spe.disconnect();
                    streams.borrow_mut().remove(proxy_id);
                })
                .register();

            let mut proxies = proxies.borrow_mut();
            proxies.add_proxy_t(proxy_spe, listener_spe);
            proxies.add_proxy_listener(proxy_id, listener);

            if let Some((stream_spe, listener_spe)) = s {
                let mut streams = streams.borrow_mut();
                streams.add_stream(proxy_id, stream_spe, listener_spe);
            }
        })
        .register();

    main_loop.run();

    Ok(())
}
