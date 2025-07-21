//! Event-based wrapper around pipewire-rs.
mod client;
mod command;
mod deserialize;
mod device;
mod event;
mod event_sender;
mod execute;
mod link;
pub mod media_class;
mod metadata;
mod node;
mod object_id;
mod property_store;
mod proxy_registry;
mod session;
pub mod state;
mod stream;
mod stream_registry;
mod sync_registry;

pub use command::{Command, CommandSender};
pub use event::{Event, StateEvent};
pub use event_sender::EventHandler;
pub use object_id::ObjectId;
pub use property_store::PropertyStore;
pub use session::Session;
