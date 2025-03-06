//! Represent valid name templating tags

use serde_with::DeserializeFromStr;

#[derive(Debug, Copy, Clone, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Tag {
    Device(DeviceTag),
    Node(NodeTag),
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

#[allow(clippy::to_string_trait_impl)] // This is not for display.
impl ToString for Tag {
    fn to_string(&self) -> String {
        match self {
            Tag::Device(DeviceTag::DeviceName) => {
                "device:device.name".to_string()
            }
            Tag::Device(DeviceTag::DeviceNick) => {
                "device:device.nick".to_string()
            }
            Tag::Device(DeviceTag::DeviceDescription) => {
                "device:device.description".to_string()
            }
            Tag::Node(NodeTag::NodeName) => "node:node.name".to_string(),
            Tag::Node(NodeTag::NodeNick) => "node:node.nick".to_string(),
            Tag::Node(NodeTag::NodeDescription) => {
                "node:node.description".to_string()
            }
            Tag::Node(NodeTag::MediaName) => "node:media.name".to_string(),
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
            _ => Err(format!("\"{}\" is not implemented", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_device_variants() {
        for device_tag in DeviceTag::iter() {
            // Do a round-trip conversion and compare results.
            let tag = Tag::Device(device_tag);
            let tag_str = tag.to_string();
            let parsed_tag: Tag = tag_str.parse().unwrap();
            assert_eq!(tag, parsed_tag);
        }
    }

    #[test]
    fn test_node_variants() {
        for node_tag in NodeTag::iter() {
            // Do a round-trip conversion and compare results.
            let tag = Tag::Node(node_tag);
            let tag_str = tag.to_string();
            let parsed_tag: Tag = tag_str.parse().unwrap();
            assert_eq!(tag, parsed_tag);
        }
    }
}
