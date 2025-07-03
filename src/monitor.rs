//! Setup and teardown of PipeWire monitoring.
//!
//! [`Client::spawn()`] starts a PipeWire monitoring thread.

mod client;
mod command;
mod deserialize;
mod device;
mod event;
mod event_sender;
mod execute;
mod link;
mod media_class;
mod metadata;
mod node;
mod object_id;
mod property_store;
mod proxy_registry;
pub mod state;
mod stream;
mod stream_registry;
mod sync_registry;

pub use command::{Command, CommandSender};
pub use event::{Event, StateEvent};
pub use event_sender::EventHandler;
pub use object_id::ObjectId;
pub use property_store::PropertyStore;

use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use nix::sys::eventfd::{EfdFlags, EventFd};
use std::os::fd::AsRawFd;

use pipewire::{
    main_loop::MainLoop, properties::properties, proxy::ProxyT,
    types::ObjectType,
};

use crate::monitor::{
    event_sender::EventSender, proxy_registry::ProxyRegistry,
    stream_registry::StreamRegistry, sync_registry::SyncRegistry,
};

/// Handle for a PipeWire monitoring thread.
///
/// On cleanup, the PipeWire [`MainLoop`](`pipewire::main_loop::MainLoop`) will
/// be notified to [`quit()`](`pipewire::main_loop::MainLoop::quit()`), and the
/// thread will be joined.
pub struct Client {
    fd: Arc<EventFd>,
    handle: Option<thread::JoinHandle<()>>,
    /// Channel for sending [`Command`]s to be executed
    tx: pipewire::channel::Sender<Command>,
}

impl Client {
    /// Spawns a thread to monitor the PipeWire instance.
    ///
    /// [`Event`]s from PipeWire are sent to the provided `handler`.
    ///
    /// Returns a [`Client`] handle for sending commands and for automatically
    /// cleaning up the thread.
    pub fn spawn<F: EventHandler>(
        remote: Option<String>,
        handler: F,
    ) -> Result<Self> {
        let shutdown_fd =
            Arc::new(EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?);

        let (tx, rx) = pipewire::channel::channel::<Command>();

        let handle = thread::spawn({
            let shutdown_fd = Arc::clone(&shutdown_fd);
            move || {
                let _ = run(remote, rx, handler, shutdown_fd);
            }
        });

        Ok(Self {
            fd: shutdown_fd,
            handle: Some(handle),
            tx,
        })
    }
}

/// Wrapper for handling PipeWire initialization/deinitialization.
fn run<F: EventHandler>(
    remote: Option<String>,
    rx: pipewire::channel::Receiver<Command>,
    handler: F,
    shutdown_fd: Arc<EventFd>,
) -> Result<()> {
    pipewire::init();

    let _guard = scopeguard::guard((), |_| unsafe {
        pipewire::deinit();
    });

    let main_loop = MainLoop::new(None)?;
    let sender = Rc::new(EventSender::new(handler, main_loop.downgrade()));

    let err_sender = Rc::clone(&sender);
    monitor_pipewire(remote, main_loop, sender, rx, shutdown_fd)
        .unwrap_or_else(move |e| {
            err_sender.send_error(e.to_string());
        });

    Ok(())
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.fd.arm();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl CommandSender for Client {
    fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    fn node_capture_start(
        &self,
        obj_id: ObjectId,
        object_serial: u64,
        capture_sink: bool,
    ) {
        let _ = self.tx.send(Command::NodeCaptureStart(
            obj_id,
            object_serial,
            capture_sink,
        ));
    }

    fn node_capture_stop(&self, obj_id: ObjectId) {
        let _ = self.tx.send(Command::NodeCaptureStop(obj_id));
    }

    fn node_mute(&self, obj_id: ObjectId, mute: bool) {
        let _ = self.tx.send(Command::NodeMute(obj_id, mute));
    }

    fn node_volumes(&self, obj_id: ObjectId, volumes: Vec<f32>) {
        let _ = self.tx.send(Command::NodeVolumes(obj_id, volumes));
    }

    fn device_mute(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
        mute: bool,
    ) {
        let _ = self.tx.send(Command::DeviceMute(
            obj_id,
            route_index,
            route_device,
            mute,
        ));
    }

    fn device_set_profile(&self, obj_id: ObjectId, profile_index: i32) {
        let _ = self
            .tx
            .send(Command::DeviceSetProfile(obj_id, profile_index));
    }

    fn device_set_route(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
    ) {
        let _ = self.tx.send(Command::DeviceSetRoute(
            obj_id,
            route_index,
            route_device,
        ));
    }

    fn device_volumes(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
        volumes: Vec<f32>,
    ) {
        let _ = self.tx.send(Command::DeviceVolumes(
            obj_id,
            route_index,
            route_device,
            volumes,
        ));
    }

    fn metadata_set_property(
        &self,
        obj_id: ObjectId,
        subject: u32,
        key: String,
        type_: Option<String>,
        value: Option<String>,
    ) {
        let _ = self.tx.send(Command::MetadataSetProperty(
            obj_id, subject, key, type_, value,
        ));
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
                        sender.send(StateEvent::StreamStopped(id));
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
                            sender.send(StateEvent::Removed(obj_id));
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
