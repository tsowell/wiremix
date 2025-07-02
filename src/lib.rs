pub mod app;
pub mod config;
pub mod device_kind;
pub mod device_widget;
pub mod dropdown_widget;
pub mod event;
pub mod help;
pub mod input;
pub mod media_class;
pub mod meter;
pub mod monitor;
pub mod node_widget;
pub mod object_list;
pub mod opt;
pub mod truncate;
pub mod view;

#[cfg(feature = "trace")]
pub mod trace;

#[cfg(test)]
mod mock {
    use crate::monitor::{Command, CommandSender, ObjectId};

    #[derive(Default)]
    pub struct MonitorHandle {}

    impl CommandSender for MonitorHandle {
        fn send(&self, _command: Command) {}
        fn node_capture_start(
            &self,
            _obj_id: ObjectId,
            _object_serial: u64,
            _capture_sink: bool,
        ) {
        }
        fn node_capture_stop(&self, _obj_id: ObjectId) {}
        fn node_mute(&self, _obj_id: ObjectId, _mute: bool) {}
        fn node_volumes(&self, _obj_id: ObjectId, _volumes: Vec<f32>) {}
        fn device_mute(
            &self,
            _obj_id: ObjectId,
            _route_index: i32,
            _route_device: i32,
            _mute: bool,
        ) {
        }
        fn device_set_profile(&self, _obj_id: ObjectId, _profile_index: i32) {}
        fn device_set_route(
            &self,
            _obj_id: ObjectId,
            _route_index: i32,
            _route_device: i32,
        ) {
        }
        fn device_volumes(
            &self,
            _obj_id: ObjectId,
            _route_index: i32,
            _route_device: i32,
            _volumes: Vec<f32>,
        ) {
        }
        fn metadata_set_property(
            &self,
            _obj_id: ObjectId,
            _subject: u32,
            _key: String,
            _type_: Option<String>,
            _value: Option<String>,
        ) {
        }
    }
}
