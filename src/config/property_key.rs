//! An identifier for a property on an object or a linked object

use serde_with::DeserializeFromStr;

use crate::wirehose::state;

#[derive(Debug, Clone, Hash, PartialEq, Eq, DeserializeFromStr)]
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

pub trait PropertyResolver {
    fn resolve_key<'a>(
        &'a self,
        state: &'a state::State,
        key: &PropertyKey,
    ) -> Option<&'a str>;
}

impl PropertyResolver for state::Device {
    /// Resolve a key using Device.
    fn resolve_key<'a>(
        &'a self,
        _state: &'a state::State,
        key: &PropertyKey,
    ) -> Option<&'a str> {
        match key {
            PropertyKey::Device(s) | PropertyKey::Bare(s) => self.props.raw(s),
            PropertyKey::Node(_) => None,
            PropertyKey::Client(_) => None,
        }
    }
}

impl PropertyResolver for state::Node {
    /// Resolve a key using Node. Falls back on resolving using the linked
    /// Device, if present.
    fn resolve_key<'a>(
        &'a self,
        state: &'a state::State,
        key: &PropertyKey,
    ) -> Option<&'a str> {
        match key {
            PropertyKey::Node(s) | PropertyKey::Bare(s) => self.props.raw(s),
            PropertyKey::Device(_) => {
                let device = state.devices.get(self.props.device_id()?)?;
                device.resolve_key(state, key)
            }
            PropertyKey::Client(_) => {
                let client = state.clients.get(self.props.client_id()?)?;
                client.resolve_key(state, key)
            }
        }
    }
}

impl PropertyResolver for state::Client {
    /// Resolve a key using Client.
    fn resolve_key<'a>(
        &'a self,
        _state: &'a state::State,
        key: &PropertyKey,
    ) -> Option<&'a str> {
        match key {
            PropertyKey::Client(s) | PropertyKey::Bare(s) => self.props.raw(s),
            PropertyKey::Node(_) => None,
            PropertyKey::Device(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_bare() {
        assert_eq!(
            PropertyKey::from_str("volume").unwrap(),
            PropertyKey::Bare("volume".into())
        );
    }

    #[test]
    fn parse_device() {
        assert_eq!(
            PropertyKey::from_str("device:alsa.name").unwrap(),
            PropertyKey::Device("alsa.name".into())
        );
    }

    #[test]
    fn parse_node() {
        assert_eq!(
            PropertyKey::from_str("node:media.class").unwrap(),
            PropertyKey::Node("media.class".into())
        );
    }

    #[test]
    fn parse_client() {
        assert_eq!(
            PropertyKey::from_str("client:application.name").unwrap(),
            PropertyKey::Client("application.name".into())
        );
    }

    #[test]
    fn empty_bare_is_error() {
        assert!(PropertyKey::from_str("").is_err());
    }

    #[test]
    fn empty_prefixed_is_error() {
        assert!(PropertyKey::from_str("device:").is_err());
        assert!(PropertyKey::from_str("node:").is_err());
        assert!(PropertyKey::from_str("client:").is_err());
    }

    #[test]
    fn roundtrip_bare() {
        let key = PropertyKey::from_str("volume").unwrap();
        assert_eq!(PropertyKey::from_str(&key.to_string()).unwrap(), key);
    }

    #[test]
    fn roundtrip_prefixed() {
        for input in ["device:foo", "node:bar", "client:baz"] {
            let key = PropertyKey::from_str(input).unwrap();
            assert_eq!(key.to_string(), input);
        }
    }

    #[test]
    fn to_string_formats() {
        assert_eq!(PropertyKey::Device("x".into()).to_string(), "device:x");
        assert_eq!(PropertyKey::Node("x".into()).to_string(), "node:x");
        assert_eq!(PropertyKey::Client("x".into()).to_string(), "client:x");
        assert_eq!(PropertyKey::Bare("x".into()).to_string(), "x");
    }

    #[test]
    fn unknown_prefix_is_bare() {
        // "other:foo" doesn't match any known prefix, so it's Bare
        assert_eq!(
            PropertyKey::from_str("other:foo").unwrap(),
            PropertyKey::Bare("other:foo".into())
        );
    }
}
