use std::collections::HashMap;

use libspa::param::ParamType;

#[derive(Default)]
pub struct DeviceStatus {
    route: bool,
    enum_route: bool,
    profile: bool,
    enum_profile: bool,
}

impl DeviceStatus {
    pub fn set(&mut self, flag: ParamType) {
        match flag {
            ParamType::Route => self.route = true,
            ParamType::EnumRoute => self.enum_route = true,
            ParamType::Profile => self.profile = true,
            ParamType::EnumProfile => self.enum_profile = true,
            _ => (),
        }
    }

    pub fn get(&self, flag: ParamType) -> bool {
        match flag {
            ParamType::Route => self.route,
            ParamType::EnumRoute => self.enum_route,
            ParamType::Profile => self.profile,
            ParamType::EnumProfile => self.enum_profile,
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct DeviceStatusTracker {
    statuses: HashMap<u32, DeviceStatus>,
}

impl DeviceStatusTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, id: u32, flag: ParamType) {
        self.statuses.entry(id).or_default().set(flag);
    }

    pub fn get(&self, id: u32) -> Option<&DeviceStatus> {
        self.statuses.get(&id)
    }
}
