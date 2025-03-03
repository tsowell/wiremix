//! Type for representing PipeWire object IDs.

use libspa::utils::dict::DictRef;
use pipewire::registry::GlobalObject;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ObjectId(u32);

impl From<&GlobalObject<&DictRef>> for ObjectId {
    fn from(obj: &GlobalObject<&DictRef>) -> Self {
        ObjectId(obj.id)
    }
}

impl From<ObjectId> for u32 {
    fn from(id: ObjectId) -> u32 {
        id.0
    }
}

#[allow(clippy::to_string_trait_impl)] // This isn't for end-users
impl ToString for ObjectId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl ObjectId {
    pub fn from_raw_id(id: u32) -> Self {
        ObjectId(id)
    }
}
