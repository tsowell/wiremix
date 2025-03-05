//! Represent valid name templating tags

pub enum Tag {
    Device(DeviceTag),
    Node(NodeTag),
}

pub enum DeviceTag {
    DeviceName,
    DeviceNick,
    DeviceDescription,
}

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

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTagError;

impl std::str::FromStr for Tag {
    type Err = ParseTagError;

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
            _ => Err(ParseTagError),
        }
    }
}
