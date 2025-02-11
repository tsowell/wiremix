use std::collections::HashMap;

use libspa::param::ParamType;

use crate::object::ObjectId;

/// Tracks whether or not a device's params have been received yet.
/// Profile and EnumProfile in particular are emitted on every Route change
/// which can get a bit spammy.
#[derive(Default)]
pub struct DeviceStatus {
    profile: bool,
    enum_profile: bool,
}

impl DeviceStatus {
    pub fn set(&mut self, flag: ParamType) {
        match flag {
            ParamType::Profile => self.profile = true,
            ParamType::EnumProfile => self.enum_profile = true,
            _ => (),
        }
    }

    pub fn get(&self, flag: ParamType) -> bool {
        match flag {
            ParamType::Profile => self.profile,
            ParamType::EnumProfile => self.enum_profile,
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct DeviceStatusTracker {
    statuses: HashMap<ObjectId, DeviceStatus>,
}

impl DeviceStatusTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, id: ObjectId, flag: ParamType) {
        self.statuses.entry(id).or_default().set(flag);
    }

    pub fn get(&self, id: ObjectId) -> Option<&DeviceStatus> {
        self.statuses.get(&id)
    }

    pub fn remove(&mut self, id: &ObjectId) -> Option<DeviceStatus> {
        self.statuses.remove(id)
    }
}
