//! Implementation for [`Names`](`crate::config::Names`). Defines default name
//! templates and handles resolving templates into strings.

use crate::config;
use crate::wirehose::state;

pub use crate::config::name_template::NameTemplate;
pub use crate::config::tag::Tag;
use crate::config::Names;
use crate::wirehose::media_class;

impl Names {
    pub fn default_stream() -> Vec<NameTemplate> {
        vec!["{node:node.name}: {node:media.name}".parse().unwrap()]
    }

    pub fn default_endpoint() -> Vec<NameTemplate> {
        vec![
            "{device:device.nick}".parse().unwrap(),
            "{node:node.description}".parse().unwrap(),
        ]
    }

    pub fn default_device() -> Vec<NameTemplate> {
        vec![
            "{device:device.nick}".parse().unwrap(),
            "{device:device.description}".parse().unwrap(),
        ]
    }

    /// Tries to resolve an object's name.
    ///
    /// Returns a name using the first template string that can be successfully
    /// resolved using the resolver.
    ///
    /// Precedence is:
    ///
    /// 1. Overrides
    /// 2. Stream/endpoint/device default templates
    /// 3. Fallback
    pub fn resolve<T: TagResolver + NameResolver>(
        &self,
        state: &state::State,
        resolver: &T,
    ) -> Option<String> {
        resolver
            .templates(state, self)
            .iter()
            .find_map(|template| {
                template.render(|tag| resolver.resolve_tag(state, tag))
            })
            .or(resolver.fallback().cloned())
    }
}

impl Default for Names {
    fn default() -> Self {
        Self {
            stream: Self::default_stream(),
            endpoint: Self::default_endpoint(),
            device: Self::default_device(),
            overrides: Vec::new(),
        }
    }
}

pub trait TagResolver {
    fn resolve_tag<'a>(
        &'a self,
        state: &'a state::State,
        tag: &Tag,
    ) -> Option<&'a str>;
}

pub trait NameResolver: TagResolver {
    fn fallback(&self) -> Option<&String>;

    fn templates<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<NameTemplate>;

    fn name_override<'a>(
        &self,
        state: &state::State,
        overrides: &'a [config::NameOverride],
        override_type: config::OverrideType,
    ) -> Option<&'a Vec<NameTemplate>> {
        overrides.iter().find_map(|name_override| {
            (name_override.types.contains(&override_type)
                && self.resolve_tag(state, &name_override.property)
                    == Some(&name_override.value))
            .then_some(&name_override.templates)
        })
    }
}

impl TagResolver for state::Device {
    /// Resolve a tag using Device.
    fn resolve_tag<'a>(
        &'a self,
        _state: &'a state::State,
        tag: &Tag,
    ) -> Option<&'a str> {
        match tag {
            Tag::Device(s) => self.props.raw(s),
            Tag::Node(_) => None,
            Tag::Client(_) => None,
        }
    }
}

impl NameResolver for state::Device {
    fn fallback(&self) -> Option<&String> {
        self.props.device_name()
    }

    fn templates<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<NameTemplate> {
        self.name_override(
            state,
            &names.overrides,
            config::OverrideType::Device,
        )
        .unwrap_or(&names.device)
    }
}

impl TagResolver for state::Node {
    /// Resolve a tag using Node. Falls back on resolving using the linked
    /// Device, if present.
    fn resolve_tag<'a>(
        &'a self,
        state: &'a state::State,
        tag: &Tag,
    ) -> Option<&'a str> {
        match tag {
            Tag::Node(s) => self.props.raw(s),
            Tag::Device(_) => {
                let device = state.devices.get(self.props.device_id()?)?;
                device.resolve_tag(state, tag)
            }
            Tag::Client(_) => {
                let client = state.clients.get(self.props.client_id()?)?;
                client.resolve_tag(state, tag)
            }
        }
    }
}

impl NameResolver for state::Node {
    fn fallback(&self) -> Option<&String> {
        self.props.node_name()
    }

    fn templates<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<NameTemplate> {
        match self.props.media_class() {
            Some(media_class)
                if media_class::is_sink(media_class)
                    || media_class::is_source(media_class) =>
            {
                self.name_override(
                    state,
                    &names.overrides,
                    config::OverrideType::Endpoint,
                )
                .unwrap_or(&names.endpoint)
            }
            _ => self
                .name_override(
                    state,
                    &names.overrides,
                    config::OverrideType::Stream,
                )
                .unwrap_or(&names.stream),
        }
    }
}

impl TagResolver for state::Client {
    /// Resolve a tag using Client.
    fn resolve_tag<'a>(
        &'a self,
        _state: &'a state::State,
        tag: &Tag,
    ) -> Option<&'a str> {
        match tag {
            Tag::Client(s) => self.props.raw(s),
            Tag::Node(_) => None,
            Tag::Device(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{NameOverride, Names, OverrideType};
    use crate::mock;
    use crate::wirehose::{state::State, ObjectId, PropertyStore, StateEvent};

    #[test]
    fn default_stream() {
        // Just make sure this doesn't panic.
        let _ = Names::default_stream();
    }

    #[test]
    fn default_endpoint() {
        // Just make sure this doesn't panic.
        let _ = Names::default_endpoint();
    }

    #[test]
    fn default_device() {
        // Just make sure this doesn't panic.
        let _ = Names::default_device();
    }

    struct Fixture {
        state: State,
        device_id: ObjectId,
        node_id: ObjectId,
        client_id: ObjectId,
        node_props: PropertyStore,
    }

    impl Fixture {
        fn new(wirehose: &mock::WirehoseHandle) -> Self {
            let mut state = State::default();

            let device_id = ObjectId::from_raw_id(0);
            let node_id = ObjectId::from_raw_id(1);
            let client_id = ObjectId::from_raw_id(2);

            let mut device_props = PropertyStore::default();
            device_props.set_device_name(String::from("Device name"));
            device_props.set_device_nick(String::from("Device nick"));
            let device_props = device_props;

            let mut node_props = PropertyStore::default();
            node_props.set_node_name(String::from("Node name"));
            node_props.set_node_nick(String::from("Node nick"));
            let node_props = node_props;

            let mut client_props = PropertyStore::default();
            client_props.set_application_name(String::from("Client name"));
            let client_props = client_props;

            let events = vec![
                StateEvent::DeviceProperties(device_id, device_props.clone()),
                StateEvent::NodeProperties(node_id, node_props.clone()),
                StateEvent::ClientProperties(client_id, client_props.clone()),
            ];

            for event in events {
                state.update(wirehose, event);
            }

            Self {
                state,
                device_id,
                node_id,
                client_id,
                node_props,
            }
        }
    }

    #[test]
    fn render_endpoint() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture
            .node_props
            .set_media_class(String::from("Audio/Sink"));
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            endpoint: vec!["{node:node.nick}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node nick")))
    }

    #[test]
    fn render_endpoint_missing_tag() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture
            .node_props
            .set_media_class(String::from("Audio/Sink"));
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            endpoint: vec!["{node:node.description}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        // Should fall back to node name
        assert_eq!(result, Some(String::from("Node name")))
    }

    #[test]
    fn render_device_missing_tag() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            device: vec!["{device:device.description}".parse().unwrap()],
            ..Default::default()
        };

        let device = fixture.state.devices.get(&fixture.device_id).unwrap();
        let result = names.resolve(&fixture.state, device);
        // Should fall back to device name
        assert_eq!(result, Some(String::from("Device name")))
    }

    #[test]
    fn render_endpoint_linked_device() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture
            .node_props
            .set_media_class(String::from("Audio/Sink"));
        fixture.node_props.set_device_id(fixture.device_id);
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            endpoint: vec!["{device:device.nick}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Device nick")))
    }

    #[test]
    fn render_endpoint_linked_device_missing_tag() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture
            .node_props
            .set_media_class(String::from("Audio/Sink"));
        fixture.node_props.set_device_id(fixture.device_id);
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            endpoint: vec!["{device:device.description}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        // Should fall back to node name
        assert_eq!(result, Some(String::from("Node name")))
    }

    #[test]
    fn render_endpoint_no_linked_device() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture
            .node_props
            .set_media_class(String::from("Audio/Sink"));
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            endpoint: vec!["{device:device.nick}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        // Should fall back to node name
        assert_eq!(result, Some(String::from("Node name")))
    }

    #[test]
    fn render_stream() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            stream: vec!["{node:node.nick}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node nick")))
    }

    #[test]
    fn render_stream_linked_client() {
        let wirehose = mock::WirehoseHandle::default();
        let mut fixture = Fixture::new(&wirehose);

        fixture.node_props.set_client_id(fixture.client_id);
        fixture.state.update(
            &wirehose,
            StateEvent::NodeProperties(fixture.node_id, fixture.node_props),
        );

        let names = Names {
            stream: vec!["{client:application.name}".parse().unwrap()],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Client name")))
    }

    #[test]
    fn render_precedence() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            stream: vec![
                "{node:node.description}".parse().unwrap(),
                "{node:node.nick}".parse().unwrap(),
            ],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node nick")))
    }

    #[test]
    fn render_override_match() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            overrides: vec![NameOverride {
                types: vec![OverrideType::Device, OverrideType::Stream],
                property: Tag::Node(String::from("node.name")),
                value: String::from("Node name"),
                templates: vec![
                    "{node:node.description}".parse().unwrap(),
                    "{node:node.nick}".parse().unwrap(),
                ],
            }],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node nick")))
    }

    #[test]
    fn render_override_type_mismatch() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            overrides: vec![NameOverride {
                types: vec![OverrideType::Device],
                property: Tag::Node(String::from("node.name")),
                value: String::from("Node name"),
                templates: vec!["{node:node.nick}".parse().unwrap()],
            }],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node name")))
    }

    #[test]
    fn render_override_value_mismatch() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            overrides: vec![NameOverride {
                types: vec![OverrideType::Device],
                property: Tag::Node(String::from("node.description")),
                value: String::from("Node name"),
                templates: vec!["{node:node.nick}".parse().unwrap()],
            }],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node name")))
    }

    #[test]
    fn render_override_empty_templates() {
        let wirehose = mock::WirehoseHandle::default();
        let fixture = Fixture::new(&wirehose);

        let names = Names {
            overrides: vec![NameOverride {
                types: vec![OverrideType::Device, OverrideType::Stream],
                property: Tag::Node(String::from("node.name")),
                value: String::from("Node name"),
                templates: vec![],
            }],
            ..Default::default()
        };

        let node = fixture.state.nodes.get(&fixture.node_id).unwrap();
        let result = names.resolve(&fixture.state, node);
        assert_eq!(result, Some(String::from("Node name")))
    }
}
