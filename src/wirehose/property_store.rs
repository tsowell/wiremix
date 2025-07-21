use std::collections::HashMap;
use std::str::FromStr;

use libspa::utils::dict::DictRef;

use anyhow::{anyhow, Result};

use crate::wirehose::ObjectId;

#[derive(Debug, Clone)]
enum PropertyValue {
    String,
    Bool(bool),
    U32(u32),
    U64(u64),
    I32(i32),
    ObjectId(ObjectId),
}

#[derive(Debug, Clone)]
struct PropertyEntry {
    raw: String,
    parsed: PropertyValue,
}

/// Stores the "info.props" properties of a PipeWire object.
///
/// Provides typed accessors for supported standard PipeWire properties.
/// [PropertyStore::raw] can be used to access any property (including
/// unsupported ones) as an unparsed string.
#[derive(Default, Debug, Clone)]
pub struct PropertyStore {
    properties: HashMap<String, PropertyEntry>,
}

impl From<String> for PropertyValue {
    fn from(_value: String) -> Self {
        PropertyValue::String
    }
}

impl From<bool> for PropertyValue {
    fn from(value: bool) -> Self {
        PropertyValue::Bool(value)
    }
}

impl From<u32> for PropertyValue {
    fn from(value: u32) -> Self {
        PropertyValue::U32(value)
    }
}

impl From<u64> for PropertyValue {
    fn from(value: u64) -> Self {
        PropertyValue::U64(value)
    }
}

impl From<i32> for PropertyValue {
    fn from(value: i32) -> Self {
        PropertyValue::I32(value)
    }
}

impl From<ObjectId> for PropertyValue {
    fn from(value: ObjectId) -> Self {
        PropertyValue::ObjectId(value)
    }
}

trait PropertyValueAccess<T> {
    fn get_value(&self) -> Option<&T>;
}

impl PropertyValueAccess<String> for PropertyEntry {
    fn get_value(&self) -> Option<&String> {
        match &self.parsed {
            PropertyValue::String => Some(&self.raw),
            _ => None,
        }
    }
}

impl PropertyValueAccess<bool> for PropertyEntry {
    fn get_value(&self) -> Option<&bool> {
        match &self.parsed {
            PropertyValue::Bool(u) => Some(u),
            _ => None,
        }
    }
}

impl PropertyValueAccess<u32> for PropertyEntry {
    fn get_value(&self) -> Option<&u32> {
        match &self.parsed {
            PropertyValue::U32(u) => Some(u),
            _ => None,
        }
    }
}

impl PropertyValueAccess<u64> for PropertyEntry {
    fn get_value(&self) -> Option<&u64> {
        match &self.parsed {
            PropertyValue::U64(u) => Some(u),
            _ => None,
        }
    }
}

impl PropertyValueAccess<i32> for PropertyEntry {
    fn get_value(&self) -> Option<&i32> {
        match &self.parsed {
            PropertyValue::I32(i) => Some(i),
            _ => None,
        }
    }
}

impl PropertyValueAccess<ObjectId> for PropertyEntry {
    fn get_value(&self) -> Option<&ObjectId> {
        match &self.parsed {
            PropertyValue::ObjectId(id) => Some(id),
            _ => None,
        }
    }
}

macro_rules! define_properties {
    ($($name:ident: $type:ty = $key:literal),* $(,)?) => {
        fn parse_dict_item(key: &str, raw: &str) -> Result<PropertyEntry> {
            match key {
                $(
                    $key => {
                        let parsed: $type = raw.parse().map_err(|_| {
                            anyhow!(
                                "Failed to parse '{}' as '{}'",
                                raw,
                                stringify!($type)
                            )
                        })?;
                        Ok(PropertyEntry {
                            raw: String::from(raw),
                            parsed: parsed.into()
                        })
                    }
                )*
                _ => Err(anyhow!("Unknown key '{}'", key))
            }
        }

        impl PropertyStore {
            $(
                #[doc = "Get a reference to the parsed "]
                #[doc = stringify!($key)]
                #[doc = " property."]
                pub fn $name(&self) -> Option<&$type> {
                    self.properties
                        .get($key)
                        .and_then(|entry| entry.get_value())
                }

                #[cfg(test)]
                paste::paste! {
                    pub fn [<set_ $name>](&mut self, value: $type) {
                        self.properties.insert(
                            String::from($key),
                            PropertyEntry {
                                raw: value.to_string(),
                                parsed: value.into(),
                            },
                        );
                    }
                }
            )*
        }

        // Ensure that all property identifiers match their keys.
        #[cfg(test)]
        mod property_tests {
            #[test]
            fn ident_and_key_match() {
                $(
                    assert_eq!(
                        stringify!($name),
                        $key.replace(&['.', '-'], "_")
                    );
                )*
            }
        }
    }
}

impl From<&DictRef> for PropertyStore {
    fn from(dict: &DictRef) -> Self {
        let mut properties = HashMap::default();
        for (key, value) in dict.iter() {
            let entry =
                parse_dict_item(key, value).unwrap_or_else(|_| PropertyEntry {
                    raw: value.to_string(),
                    parsed: PropertyValue::String,
                });
            properties.insert(String::from(key), entry);
        }
        PropertyStore { properties }
    }
}

impl PropertyStore {
    /// Get the raw string value for a property.
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|e| e.raw.as_str())
    }
}

impl FromStr for ObjectId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str(s).map(ObjectId::from_raw_id)
    }
}

define_properties! {
    // Key used by wireplumber
    card_profile_device: i32 = "card.profile.device",

    // Keys from src/pipewire/keys.h
    pipewire_protocol: String = "pipewire.protocol",
    pipewire_access: String = "pipewire.access",
    pipewire_client_access: String = "pipewire.client.access",
    pipewire_sec_pid: i32 = "pipewire.sec.pid",
    pipewire_sec_uid: u32 = "pipewire.sec.uid",
    pipewire_sec_gid: u32 = "pipewire.sec.gid",
    pipewire_sec_label: String = "pipewire.sec.label",
    pipewire_sec_socket: String = "pipewire.sec.socket",
    pipewire_sec_engine: String = "pipewire.sec.engine",
    pipewire_sec_app_id: String = "pipewire.sec.app-id",
    pipewire_sec_instance_id: String = "pipewire.sec.instance-id",
    library_name_system: String = "library.name.system",
    library_name_loop: String = "library.name.loop",
    library_name_dbus: String = "library.name.dbus",
    object_path: String = "object.path",
    object_id: ObjectId = "object.id",
    object_serial: u64 = "object.serial",
    object_linger: bool = "object.linger",
    object_register: bool = "object.register",
    object_export: bool = "object.export",
    config_prefix: String = "config.prefix",
    config_name: String = "config.name",
    config_override_prefix: String = "config.override.prefix",
    config_override_name: String = "config.override.name",
    loop_name: String = "loop.name",
    loop_class: String = "loop.class",
    loop_rt_prio: i32 = "loop.rt-prio",
    loop_cancel: bool = "loop.cancel",
    context_user_name: String = "context.user-name",
    context_host_name: String = "context.host-name",
    core_name: String = "core.name",
    core_version: String = "core.version",
    core_daemon: bool = "core.daemon",
    cpu_max_align: u32 = "cpu.max-align",
    priority_session: i32 = "priority.session",
    priority_driver: i32 = "priority.driver",
    remote_name: String = "remote.name",
    remote_intention: String = "remote.intention",
    application_name: String = "application.name",
    application_id: String = "application.id",
    application_version: String = "application.version",
    application_icon: String = "application.icon",
    application_icon_name: String = "application.icon-name",
    application_language: String = "application.language",
    application_process_id: u64 = "application.process.id",
    application_process_binary: String = "application.process.binary",
    application_process_user: String = "application.process.user",
    application_process_host: String = "application.process.host",
    application_process_machine_id: String = "application.process.machine-id",
    application_process_session_id: ObjectId = "application.process.session-id",
    window_x11_display: String = "window.x11.display",
    client_id: ObjectId = "client.id",
    client_name: String = "client.name",
    client_api: String = "client.api",
    node_id: ObjectId = "node.id",
    node_name: String = "node.name",
    node_nick: String = "node.nick",
    node_description: String = "node.description",
    node_plugged: u64 = "node.plugged",
    node_session: ObjectId = "node.session",
    node_group: String = "node.group",
    node_sync_group: String = "node.sync-group",
    node_sync: bool = "node.sync",
    node_transport: bool = "node.transport",
    node_exclusive: bool = "node.exclusive",
    node_autoconnect: bool = "node.autoconnect",
    node_latency: String = "node.latency",
    node_max_latency: String = "node.max-latency",
    node_lock_quantum: bool = "node.lock-quantum",
    node_force_quantum: u32 = "node.force-quantum",
    node_rate: String = "node.rate",
    node_lock_rate: bool = "node.lock-rate",
    node_force_rate: u32 = "node.force-rate",
    node_dont_reconnect: bool = "node.dont-reconnect",
    node_always_process: bool = "node.always-process",
    node_want_driver: bool = "node.want-driver",
    node_pause_on_idle: bool = "node.pause-on-idle",
    node_suspend_on_idle: bool = "node.suspend-on-idle",
    node_cache_params: bool = "node.cache-params",
    node_transport_sync: bool = "node.transport.sync",
    node_driver: bool = "node.driver",
    node_driver_id: ObjectId = "node.driver-id",
    node_async: bool = "node.async",
    node_loop_name: String = "node.loop.name",
    node_loop_class: String = "node.loop.class",
    node_stream: bool = "node.stream",
    node_virtual: bool = "node.virtual",
    node_passive: bool = "node.passive",
    node_link_group: String = "node.link-group",
    node_network: bool = "node.network",
    node_trigger: bool = "node.trigger",
    node_channel_names: String = "node.channel-names",
    node_device_port_name_prefix: String = "node.device-port-name-prefix",
    port_id: ObjectId = "port.id",
    port_name: String = "port.name",
    port_direction: String = "port.direction",
    port_alias: String = "port.alias",
    port_physical: bool = "port.physical",
    port_terminal: bool = "port.terminal",
    port_control: bool = "port.control",
    port_monitor: bool = "port.monitor",
    port_cache_params: bool = "port.cache-params",
    port_extra: String = "port.extra",
    port_passive: bool = "port.passive",
    port_ignore_latency: bool = "port.ignore-latency",
    port_group: String = "port.group",
    link_id: ObjectId = "link.id",
    link_input_node: ObjectId = "link.input.node",
    link_input_port: ObjectId = "link.input.port",
    link_output_node: ObjectId = "link.output.node",
    link_output_port: ObjectId = "link.output.port",
    link_passive: bool = "link.passive",
    link_feedback: bool = "link.feedback",
    link_async: bool = "link.async",
    device_id: ObjectId = "device.id",
    device_name: String = "device.name",
    device_plugged: u64 = "device.plugged",
    device_nick: String = "device.nick",
    device_string: String = "device.string",
    device_api: String = "device.api",
    device_description: String = "device.description",
    device_bus_path: String = "device.bus-path",
    device_serial: String = "device.serial",
    device_vendor_id: String = "device.vendor.id",
    device_vendor_name: String = "device.vendor.name",
    device_product_id: String = "device.product.id",
    device_product_name: String = "device.product.name",
    device_class: String = "device.class",
    device_form_factor: String = "device.form-factor",
    device_bus: String = "device.bus",
    device_subsystem: String = "device.subsystem",
    device_sysfs_path: String = "device.sysfs.path",
    device_icon: String = "device.icon",
    device_icon_name: String = "device.icon-name",
    device_intended_roles: String = "device.intended-roles",
    device_cache_params: bool = "device.cache-params",
    module_id: ObjectId = "module.id",
    module_name: String = "module.name",
    module_author: String = "module.author",
    module_description: String = "module.description",
    module_usage: String = "module.usage",
    module_version: String = "module.version",
    module_deprecated: String = "module.deprecated",
    factory_id: ObjectId = "factory.id",
    factory_name: String = "factory.name",
    factory_usage: String = "factory.usage",
    factory_type_name: String = "factory.type.name",
    factory_type_version: u32 = "factory.type.version",
    stream_is_live: bool = "stream.is-live",
    stream_latency_min: String = "stream.latency.min",
    stream_latency_max: String = "stream.latency.max",
    stream_monitor: bool = "stream.monitor",
    stream_dont_remix: bool = "stream.dont-remix",
    stream_capture_sink: bool = "stream.capture.sink",
    media_type: String = "media.type",
    media_category: String = "media.category",
    media_role: String = "media.role",
    media_class: String = "media.class",
    media_name: String = "media.name",
    media_title: String = "media.title",
    media_artist: String = "media.artist",
    media_album: String = "media.album",
    media_copyright: String = "media.copyright",
    media_software: String = "media.software",
    media_language: String = "media.language",
    media_filename: String = "media.filename",
    media_icon: String = "media.icon",
    media_icon_name: String = "media.icon-name",
    media_comment: String = "media.comment",
    media_date: String = "media.date",
    media_format: u32 = "media.format",
    format_dsp: String = "format.dsp",
    audio_channel: String = "audio.channel",
    audio_rate: u32 = "audio.rate",
    audio_channels: u32 = "audio.channels",
    audio_format: String = "audio.format",
    audio_allowed_rates: String = "audio.allowed-rates",
    target_object: String = "target.object",
}
