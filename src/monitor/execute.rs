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
        Command::NodeVolumes(obj_id, volumes) => {
            if let Some(node) = proxies.get_node(obj_id) {
                node_set_volumes(node, volumes);
            }
        }
        Command::DeviceVolumes(obj_id, route_index, route_device, volumes) => {
            if let Some(device) = proxies.get_device(obj_id) {
                device_set_volumes(device, route_index, route_device, volumes);
            }
        }
        Command::NodeCapture(obj_id, object_serial, capture_sink) => {
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
        Command::MetadataSetProperty(obj_id, subject, key, type_, value) => {
            if let Some(metadata) = proxies.get_metadata(obj_id) {
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

fn node_set_volumes(node: &Node, volumes: Vec<f32>) {
    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_Props,
            id: libspa_sys::SPA_PARAM_Props,
            properties: vec![
                Property {
                    key: libspa_sys::SPA_PROP_channelVolumes,
                    flags: PropertyFlags::empty(),
                    value: Value::ValueArray(ValueArray::Float(
                        volumes.clone(),
                    )),
                },
                Property {
                    key: libspa_sys::SPA_PROP_softVolumes,
                    flags: PropertyFlags::empty(),
                    value: Value::ValueArray(ValueArray::Float(volumes)),
                },
            ],
        }),
    )
    .unwrap()
    .0
    .into_inner();

    node.set_param(ParamType::Props, 0, Pod::from_bytes(&values).unwrap());
}

fn device_set_volumes(
    device: &Device,
    route_index: i32,
    route_device: i32,
    volumes: Vec<f32>,
) {
    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_ParamRoute,
            id: libspa_sys::SPA_PARAM_Route,
            properties: vec![
                Property {
                    key: libspa_sys::SPA_PARAM_ROUTE_index,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(route_index),
                },
                Property {
                    key: libspa_sys::SPA_PARAM_ROUTE_device,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(route_device),
                },
                Property {
                    key: libspa_sys::SPA_PARAM_ROUTE_props,
                    flags: PropertyFlags::empty(),
                    value: Value::Object(Object {
                        type_: libspa_sys::SPA_TYPE_OBJECT_Props,
                        id: libspa_sys::SPA_PARAM_Route,
                        properties: vec![
                            Property {
                                key: libspa_sys::SPA_PROP_channelVolumes,
                                flags: PropertyFlags::empty(),
                                value: Value::ValueArray(ValueArray::Float(
                                    volumes.clone(),
                                )),
                            },
                            Property {
                                key: libspa_sys::SPA_PROP_softVolumes,
                                flags: PropertyFlags::empty(),
                                value: Value::ValueArray(ValueArray::Float(
                                    volumes,
                                )),
                            },
                        ],
                    }),
                },
                Property {
                    key: libspa_sys::SPA_PARAM_ROUTE_save,
                    flags: PropertyFlags::empty(),
                    value: Value::Bool(true),
                },
            ],
        }),
    )
    .unwrap()
    .0
    .into_inner();

    device.set_param(ParamType::Route, 0, Pod::from_bytes(&values).unwrap());
}
