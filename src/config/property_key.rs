//! An identifier for a property on an object or a linked object

use serde_with::DeserializeFromStr;

#[derive(Debug, Clone, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub enum PropertyKey {
    Device(String),
    Node(String),
    Client(String),
}

#[allow(clippy::to_string_trait_impl)] // This is not for display.
impl ToString for PropertyKey {
    fn to_string(&self) -> String {
        match self {
            PropertyKey::Device(s) => {
                format!("device:{s}")
            }
            PropertyKey::Node(s) => {
                format!("node:{s}")
            }
            PropertyKey::Client(s) => {
                format!("client:{s}")
            }
        }
    }
}

impl std::str::FromStr for PropertyKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(key) = s.strip_prefix("client:") {
            Ok(PropertyKey::Client(String::from(key)))
        } else if let Some(key) = s.strip_prefix("device:") {
            Ok(PropertyKey::Device(String::from(key)))
        } else if let Some(key) = s.strip_prefix("node:") {
            Ok(PropertyKey::Node(String::from(key)))
        } else {
            Err(format!("\"{s}\" is not implemented"))
        }
    }
}
