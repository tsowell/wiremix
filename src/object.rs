use libspa::utils::dict::DictRef;
use pipewire::registry::GlobalObject;

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ObjectId(u32);

impl From<&GlobalObject<&DictRef>> for ObjectId {
    fn from(obj: &GlobalObject<&DictRef>) -> Self {
        ObjectId(obj.id)
    }
}

impl ObjectId {
    pub fn from_raw_id(id: u32) -> Self {
        ObjectId(id)
    }
}
