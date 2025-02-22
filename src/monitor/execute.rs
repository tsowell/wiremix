use std::rc::Rc;

use crate::command::Command;
use crate::monitor::{stream, EventSender, ProxyRegistry, StreamRegistry};

use pipewire::{core::Core, device::Device, node::Node};

use libspa::param::ParamType;
use libspa::pod::{
    serialize::PodSerializer, Object, Pod, Property, PropertyFlags, Value,
    ValueArray,
};

pub fn execute_command(
    core: &Core,
    sender: Rc<EventSender>,
    streams: &mut StreamRegistry<stream::StreamData>,
    proxies: &ProxyRegistry,
    command: Command,
) {
    match command {
        Command::NodeMute(obj_id, mute) => {
            if let Some(node) = proxies.nodes.get(&obj_id) {
                node_set_mute(node, mute);
            }
        }
        Command::DeviceMute(obj_id, route_index, route_device, mute) => {
            if let Some(device) = proxies.devices.get(&obj_id) {
                device_set_mute(device, route_index, route_device, mute);
            }
        }
        Command::NodeVolumes(obj_id, volumes) => {
            if let Some(node) = proxies.nodes.get(&obj_id) {
                node_set_volumes(node, volumes);
            }
        }
        Command::DeviceVolumes(obj_id, route_index, route_device, volumes) => {
            if let Some(device) = proxies.devices.get(&obj_id) {
                device_set_volumes(device, route_index, route_device, volumes);
            }
        }
        Command::DeviceSetRoute(obj_id, route_index, route_device) => {
            if let Some(device) = proxies.devices.get(&obj_id) {
                device_set_route(device, route_index, route_device);
            }
        }
        Command::NodeCaptureStart(obj_id, object_serial, capture_sink) => {
            let result = stream::capture_node(
                core,
                &sender,
                obj_id,
                &object_serial.to_string(),
                capture_sink,
            );
            if let Some((stream, listener)) = result {
                streams.add_stream(obj_id, stream, listener);
            }
        }
        Command::NodeCaptureStop(obj_id) => {
            streams.remove(obj_id);
        }
        Command::MetadataSetProperty(obj_id, subject, key, type_, value) => {
            if let Some(metadata) = proxies.metadatas.get(&obj_id) {
                metadata.set_property(
                    subject,
                    &key,
                    type_.as_deref(),
                    value.as_deref(),
                );
            }
        }
    }
}

fn node_set_mute(node: &Node, mute: bool) {
    node_set_properties(
        node,
        vec![
            Property {
                key: libspa_sys::SPA_PROP_mute,
                flags: PropertyFlags::empty(),
                value: Value::Bool(mute),
            },
            Property {
                key: libspa_sys::SPA_PROP_mute,
                flags: PropertyFlags::empty(),
                value: Value::Bool(mute),
            },
        ],
    );
}

fn node_set_volumes(node: &Node, volumes: Vec<f32>) {
    node_set_properties(
        node,
        vec![Property {
            key: libspa_sys::SPA_PROP_channelVolumes,
            flags: PropertyFlags::empty(),
            value: Value::ValueArray(ValueArray::Float(volumes.clone())),
        }],
    );
}

fn node_set_properties(node: &Node, properties: Vec<Property>) {
    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_Props,
            id: libspa_sys::SPA_PARAM_Props,
            properties,
        }),
    )
    .unwrap()
    .0
    .into_inner();

    node.set_param(ParamType::Props, 0, Pod::from_bytes(&values).unwrap());
}

fn device_set_mute(
    device: &Device,
    route_index: i32,
    route_device: i32,
    mute: bool,
) {
    device_set_properties(
        device,
        route_index,
        route_device,
        vec![
            Property {
                key: libspa_sys::SPA_PROP_mute,
                flags: PropertyFlags::empty(),
                value: Value::Bool(mute),
            },
            Property {
                key: libspa_sys::SPA_PROP_mute,
                flags: PropertyFlags::empty(),
                value: Value::Bool(mute),
            },
        ],
    );
}

fn device_set_volumes(
    device: &Device,
    route_index: i32,
    route_device: i32,
    volumes: Vec<f32>,
) {
    device_set_properties(
        device,
        route_index,
        route_device,
        vec![Property {
            key: libspa_sys::SPA_PROP_channelVolumes,
            flags: PropertyFlags::empty(),
            value: Value::ValueArray(ValueArray::Float(volumes.clone())),
        }],
    );
}

fn device_set_route(device: &Device, route_index: i32, route_device: i32) {
    device_set_properties(device, route_index, route_device, Vec::new());
}

fn device_set_properties(
    device: &Device,
    route_index: i32,
    route_device: i32,
    properties: Vec<Property>,
) {
    let mut route_properties = Vec::new();
    route_properties.push(Property {
        key: libspa_sys::SPA_PARAM_ROUTE_index,
        flags: PropertyFlags::empty(),
        value: Value::Int(route_index),
    });
    route_properties.push(Property {
        key: libspa_sys::SPA_PARAM_ROUTE_device,
        flags: PropertyFlags::empty(),
        value: Value::Int(route_device),
    });
    if !properties.is_empty() {
        route_properties.push(Property {
            key: libspa_sys::SPA_PARAM_ROUTE_props,
            flags: PropertyFlags::empty(),
            value: Value::Object(Object {
                type_: libspa_sys::SPA_TYPE_OBJECT_Props,
                id: libspa_sys::SPA_PARAM_Route,
                properties,
            }),
        });
    }
    route_properties.push(Property {
        key: libspa_sys::SPA_PARAM_ROUTE_save,
        flags: PropertyFlags::empty(),
        value: Value::Bool(true),
    });
    let route_properties = route_properties;

    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_ParamRoute,
            id: libspa_sys::SPA_PARAM_Route,
            properties: route_properties,
        }),
    )
    .unwrap()
    .0
    .into_inner();

    device.set_param(ParamType::Route, 0, Pod::from_bytes(&values).unwrap());
}
