//! Represent valid name templating tags

use serde::{de, Deserialize, Deserializer};

#[derive(Debug, Copy, Clone)]
pub enum Tag {
    Device(DeviceTag),
    Node(NodeTag),
}

#[derive(Debug, Copy, Clone)]
pub enum DeviceTag {
    DeviceName,
    DeviceNick,
    DeviceDescription,
}

#[derive(Debug, Copy, Clone)]
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
            _ => Err("Tag doesn't exist".to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}
