//! Represent valid name templating tags

use serde_with::DeserializeFromStr;

#[derive(Debug, Copy, Clone, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Tag {
    Device(DeviceTag),
    Node(NodeTag),
    Client(ClientTag),
}

// These correspond to PipeWire property names.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq, strum::EnumIter))]
pub enum DeviceTag {
    DeviceName,
    DeviceNick,
    DeviceDescription,
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq, strum::EnumIter))]
pub enum NodeTag {
    NodeName,
    NodeNick,
    NodeDescription,
    MediaName,
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq, strum::EnumIter))]
pub enum ClientTag {
    ApplicationName,
    ApplicationProcessBinary,
}

#[allow(clippy::to_string_trait_impl)] // This is not for display.
impl ToString for Tag {
    fn to_string(&self) -> String {
        match self {
            Tag::Device(DeviceTag::DeviceName) => {
                String::from("device:device.name")
            }
            Tag::Device(DeviceTag::DeviceNick) => {
                String::from("device:device.nick")
            }
            Tag::Device(DeviceTag::DeviceDescription) => {
                String::from("device:device.description")
            }
            Tag::Node(NodeTag::NodeName) => String::from("node:node.name"),
            Tag::Node(NodeTag::NodeNick) => String::from("node:node.nick"),
            Tag::Node(NodeTag::NodeDescription) => {
                String::from("node:node.description")
            }
            Tag::Node(NodeTag::MediaName) => String::from("node:media.name"),
            Tag::Client(ClientTag::ApplicationName) => {
                String::from("client:application.name")
            }
            Tag::Client(ClientTag::ApplicationProcessBinary) => {
                String::from("client:application.process.binary")
            }
        }
    }
}

impl std::str::FromStr for Tag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "device:device.name" => Ok(Tag::Device(DeviceTag::DeviceName)),
            "device:device.nick" => Ok(Tag::Device(DeviceTag::DeviceNick)),
            "device:device.description" => {
                Ok(Tag::Device(DeviceTag::DeviceDescription))
            }
            "node:node.name" => Ok(Tag::Node(NodeTag::NodeName)),
            "node:node.nick" => Ok(Tag::Node(NodeTag::NodeNick)),
            "node:node.description" => Ok(Tag::Node(NodeTag::NodeDescription)),
            "node:media.name" => Ok(Tag::Node(NodeTag::MediaName)),
            "client:application.name" => {
                Ok(Tag::Client(ClientTag::ApplicationName))
            }
            "client:application.process.binary" => {
                Ok(Tag::Client(ClientTag::ApplicationProcessBinary))
            }
            _ => Err(format!("\"{s}\" is not implemented")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn device_variants() {
        for device_tag in DeviceTag::iter() {
            // Do a round-trip conversion and compare results.
            let tag = Tag::Device(device_tag);
            let tag_str = tag.to_string();
            let parsed_tag: Tag = tag_str.parse().unwrap();
            assert_eq!(tag, parsed_tag);
        }
    }

    #[test]
    fn node_variants() {
        for node_tag in NodeTag::iter() {
            // Do a round-trip conversion and compare results.
            let tag = Tag::Node(node_tag);
            let tag_str = tag.to_string();
            let parsed_tag: Tag = tag_str.parse().unwrap();
            assert_eq!(tag, parsed_tag);
        }
    }

    #[test]
    fn client_variants() {
        for client_tag in ClientTag::iter() {
            // Do a round-trip conversion and compare results.
            let tag = Tag::Client(client_tag);
            let tag_str = tag.to_string();
            let parsed_tag: Tag = tag_str.parse().unwrap();
            assert_eq!(tag, parsed_tag);
        }
    }
}
