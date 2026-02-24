//! An identifier for a property on an object or a linked object

use serde_with::DeserializeFromStr;

#[derive(Debug, Clone, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub enum PropertyKey {
    Device(String),
    Node(String),
    Client(String),
    Bare(String),
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
            PropertyKey::Bare(s) => s.to_string(),
        }
    }
}

impl std::str::FromStr for PropertyKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (variant, key): (fn(String) -> PropertyKey, &str) =
            if let Some(key) = s.strip_prefix("client:") {
                (PropertyKey::Client, key)
            } else if let Some(key) = s.strip_prefix("device:") {
                (PropertyKey::Device, key)
            } else if let Some(key) = s.strip_prefix("node:") {
                (PropertyKey::Node, key)
            } else {
                (PropertyKey::Bare, s)
            };

        if key.is_empty() {
            Err(format!("Empty property name in \"{s}\""))
        } else {
            Ok(variant(String::from(key)))
        }
    }
}
