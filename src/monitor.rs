//! Setup and teardown of PipeWire monitoring.
//!
//! [`spawn()`] starts a PipeWire monitoring thread.

mod client;
mod deserialize;
mod device;
mod event_sender;
mod execute;
mod link;
mod metadata;
mod node;
mod proxy_registry;
mod stream;
mod stream_registry;
mod sync_registry;

use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::thread;

use nix::sys::eventfd::{EfdFlags, EventFd};
use std::os::fd::AsRawFd;

use pipewire::{
    main_loop::MainLoop, properties::properties, proxy::ProxyT,
    types::ObjectType,
};

use crate::command::Command;
use crate::event::{Event, MonitorEvent};
use crate::monitor::{
    event_sender::EventSender, proxy_registry::ProxyRegistry,
    stream_registry::StreamRegistry, sync_registry::SyncRegistry,
};
use crate::object::ObjectId;

/// Spawns a thread to monitor the PipeWire instance.
///
/// [`Event`](`crate::event::Event`)s from PipeWire are sent to `tx`.
/// [`Command`](`crate::command::Command`)s sent to `rx` will be executed.
///
/// Returns a [`MonitorHandle`] to automatically clean up the thread.
pub fn spawn(
    remote: Option<String>,
    tx: Arc<mpsc::Sender<Event>>,
    rx: pipewire::channel::Receiver<Command>,
) -> Result<MonitorHandle> {
    let shutdown_fd =
        Arc::new(EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?);

    let handle = thread::spawn({
        let shutdown_fd = Arc::clone(&shutdown_fd);
        move || {
            let _ = run(remote, tx, rx, shutdown_fd);
        }
    });

    Ok(MonitorHandle {
        fd: Some(shutdown_fd),
        handle: Some(handle),
    })
}

/// Wrapper for handling PipeWire initialization/deinitialization.
fn run(
    remote: Option<String>,
    tx: Arc<mpsc::Sender<Event>>,
    rx: pipewire::channel::Receiver<Command>,
    shutdown_fd: Arc<EventFd>,
) -> Result<()> {
    pipewire::init();

    let _guard = scopeguard::guard((), |_| unsafe {
        pipewire::deinit();
    });

    let main_loop = MainLoop::new(None)?;
    let sender = Rc::new(EventSender::new(tx, main_loop.downgrade()));

    let err_sender = Rc::clone(&sender);
    monitor_pipewire(remote, main_loop, sender, rx, shutdown_fd)
        .unwrap_or_else(move |e| {
            err_sender.send_error(e.to_string());
        });

    Ok(())
}

/// Handle for a PipeWire monitoring thread.
///
/// On cleanup, the PipeWire [`MainLoop`](`pipewire::main_loop::MainLoop`) will
/// be notified to [`quit()`](`pipewire::main_loop::MainLoop::quit()`), and the
/// thread will be joined.
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

/// Monitors PipeWire.
///
/// Sets up core listeners and runs the PipeWire main loop.
fn monitor_pipewire(
    remote: Option<String>,
    main_loop: MainLoop,
    sender: Rc<EventSender>,
    rx: pipewire::channel::Receiver<Command>,
    shutdown_fd: Arc<EventFd>,
) -> Result<()> {
    let context = pipewire::context::Context::new(&main_loop)?;
    let props = remote.map(|remote| {
        properties! {
            *pipewire::keys::REMOTE_NAME => remote
        }
    });
    let core = Rc::new(context.connect(props)?);

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

    let syncs = Rc::new(RefCell::new(SyncRegistry::default()));

    let _core_listener = core
        .add_listener_local()
        .done({
            let sender_weak = Rc::downgrade(&sender);
            let syncs_weak = Rc::downgrade(&syncs);
            move |_id, seq| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                let Some(syncs) = syncs_weak.upgrade() else {
                    return;
                };
                if syncs.borrow_mut().done(seq) {
                    sender.send_ready();
                }
            }
        })
        .error({
            let sender_weak = Rc::downgrade(&sender);
            move |_id, _seq, _res, message| {
                if let Some(sender) = sender_weak.upgrade() {
                    sender.send_error(message.to_string());
                };
            }
        })
        .register();

    let registry = Rc::new(core.get_registry()?);
    let registry_weak = Rc::downgrade(&registry);

    // Proxies and their listeners need to stay alive so store them here
    let proxies = Rc::new(RefCell::new(ProxyRegistry::try_new()?));
    // It's not safe to delete proxies and listeners during PipeWire callbacks,
    // so registries defer cleanup and use an EventFd to signal that objects
    // are pending deletion.
    let _proxy_gc_watch = main_loop.loop_().add_io(
        proxies.borrow().gc_fd.as_raw_fd(),
        libspa::support::system::IoFlags::IN,
        {
            let proxies = Rc::clone(&proxies);
            move |_status| {
                proxies.borrow_mut().collect_garbage();
            }
        },
    );

    // Proxies and their listeners need to stay alive so store them here
    let streams = Rc::new(RefCell::new(StreamRegistry::try_new()?));
    // It's not safe to delete proxies and listeners during PipeWire callbacks,
    // so registries defer cleanup and use an EventFd to signal that objects
    // are pending deletion.
    let _streams_gc_watch = main_loop.loop_().add_io(
        streams.borrow().gc_fd.as_raw_fd(),
        libspa::support::system::IoFlags::IN,
        {
            let streams = Rc::clone(&streams);
            let sender_weak = Rc::downgrade(&sender);
            move |_status| {
                let collected = streams.borrow_mut().collect_garbage();
                if let Some(sender) = sender_weak.upgrade() {
                    for id in collected {
                        sender.send(MonitorEvent::StreamStopped(id));
                    }
                }
            }
        },
    );

    let _registry_listener = registry
        .add_listener_local()
        .global({
            let core_weak = Rc::downgrade(&core);
            let proxies = Rc::clone(&proxies);
            let sender_weak = Rc::downgrade(&sender);
            let streams_weak = Rc::downgrade(&streams);
            let syncs_weak = Rc::downgrade(&syncs);
            move |obj| {
                let obj_id = ObjectId::from(obj);
                let Some(registry) = registry_weak.upgrade() else {
                    return;
                };

                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };

                let Some(streams) = streams_weak.upgrade() else {
                    return;
                };

                let Some(core) = core_weak.upgrade() else {
                    return;
                };

                let Some(syncs) = syncs_weak.upgrade() else {
                    return;
                };

                let proxy_spe = match obj.type_ {
                    ObjectType::Client => {
                        let result =
                            client::monitor_client(&registry, obj, &sender);
                        if let Some((node, listener)) = result {
                            proxies.borrow_mut().add_client(
                                obj_id,
                                Rc::clone(&node),
                                listener,
                            );
                            Some(node as Rc<dyn ProxyT>)
                        } else {
                            None
                        }
                    }
                    ObjectType::Node => {
                        let result =
                            node::monitor_node(&registry, obj, &sender);
                        if let Some((node, listener)) = result {
                            proxies.borrow_mut().add_node(
                                obj_id,
                                Rc::clone(&node),
                                listener,
                            );
                            Some(node as Rc<dyn ProxyT>)
                        } else {
                            None
                        }
                    }
                    ObjectType::Device => {
                        let result =
                            device::monitor_device(&registry, obj, &sender);
                        match result {
                            Some((device, listener)) => {
                                proxies.borrow_mut().add_device(
                                    obj_id,
                                    Rc::clone(&device),
                                    listener,
                                );
                                Some(device as Rc<dyn ProxyT>)
                            }
                            None => None,
                        }
                    }
                    ObjectType::Link => {
                        let result =
                            link::monitor_link(&registry, obj, &sender);
                        match result {
                            Some((link, listener)) => {
                                proxies.borrow_mut().add_link(
                                    obj_id,
                                    Rc::clone(&link),
                                    listener,
                                );
                                Some(link as Rc<dyn ProxyT>)
                            }
                            None => None,
                        }
                    }
                    ObjectType::Metadata => {
                        let result =
                            metadata::monitor_metadata(&registry, obj, &sender);
                        match result {
                            Some((metadata, listener)) => {
                                proxies.borrow_mut().add_metadata(
                                    obj_id,
                                    Rc::clone(&metadata),
                                    listener,
                                );
                                Some(metadata as Rc<dyn ProxyT>)
                            }
                            None => None,
                        }
                    }
                    _ => None,
                };
                let Some(proxy_spe) = proxy_spe else {
                    return;
                };

                let proxy = proxy_spe.upcast_ref();

                // Use a weak ref to prevent references cycle between Proxy and proxies:
                // - ref on proxies in the closure, bound to the Proxy lifetime
                // - proxies owning a ref on Proxy as well
                let proxies_weak = Rc::downgrade(&proxies);
                let streams_weak = Rc::downgrade(&streams);
                let sender_weak = Rc::downgrade(&sender);
                let listener = proxy
                    .add_listener_local()
                    .removed(move || {
                        if let Some(sender) = sender_weak.upgrade() {
                            sender.send(MonitorEvent::Removed(obj_id));
                        };
                        if let Some(proxies) = proxies_weak.upgrade() {
                            proxies.borrow_mut().remove(obj_id);
                        };
                        if let Some(streams) = streams_weak.upgrade() {
                            streams.borrow_mut().remove(obj_id);
                        };
                    })
                    .register();

                proxies.borrow_mut().add_proxy_listener(obj_id, listener);

                syncs.borrow_mut().global(&core);
            }
        })
        .register();

    let proxies = Rc::clone(&proxies);
    let _receiver = rx.attach(main_loop.loop_(), {
        let core_weak = Rc::downgrade(&core);
        let sender_weak = Rc::downgrade(&sender);
        let streams_weak = Rc::downgrade(&streams);
        move |command| {
            let Some(core) = core_weak.upgrade() else {
                return;
            };
            let Some(sender) = sender_weak.upgrade() else {
                return;
            };
            let Some(streams) = streams_weak.upgrade() else {
                return;
            };
            execute::execute_command(
                &core,
                sender,
                &mut streams.borrow_mut(),
                &Rc::clone(&proxies).borrow(),
                command,
            );
        }
    });

    main_loop.run();

    Ok(())
}
