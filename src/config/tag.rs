//! Represent valid name templating tags

use serde_with::DeserializeFromStr;

#[derive(Debug, Clone, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Tag {
    Device(String),
    Node(String),
    Client(String),
}

#[allow(clippy::to_string_trait_impl)] // This is not for display.
impl ToString for Tag {
    fn to_string(&self) -> String {
        match self {
            Tag::Device(s) => {
                format!("device:{s}")
            }
            Tag::Node(s) => {
                format!("node:{s}")
            }
            Tag::Client(s) => {
                format!("client:{s}")
            }
        }
    }
}

impl std::str::FromStr for Tag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(key) = s.strip_prefix("client:") {
            Ok(Tag::Client(String::from(key)))
        } else if let Some(key) = s.strip_prefix("device:") {
            Ok(Tag::Device(String::from(key)))
        } else if let Some(key) = s.strip_prefix("node:") {
            Ok(Tag::Node(String::from(key)))
        } else {
            Err(format!("\"{s}\" is not implemented"))
        }
    }
}
