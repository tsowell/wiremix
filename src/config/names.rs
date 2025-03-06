//! Format object names using templates from configuration.

use crate::config;
use crate::state;

pub use crate::config::name_template::NameTemplate;
pub use crate::config::tag::{DeviceTag, NodeTag, Tag};

pub trait NameResolver {
    fn resolve_tag<'a>(
        &'a self,
        state: &'a state::State,
        tag: Tag,
    ) -> Option<&'a String>;

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
                && self.resolve_tag(state, name_override.property)
                    == Some(&name_override.value))
            .then_some(&name_override.templates)
        })
    }
}

impl NameResolver for state::Device {
    /// Resolve a tag using Device.
    fn resolve_tag<'a>(
        &'a self,
        _state: &'a state::State,
        tag: Tag,
    ) -> Option<&'a String> {
        match tag {
            Tag::Device(DeviceTag::DeviceName) => self.name.as_ref(),
            Tag::Device(DeviceTag::DeviceNick) => self.nick.as_ref(),
            Tag::Device(DeviceTag::DeviceDescription) => {
                self.description.as_ref()
            }
            Tag::Node(_) => None,
        }
    }

    fn fallback(&self) -> Option<&String> {
        self.name.as_ref()
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

impl NameResolver for state::Node {
    /// Resolve a tag using Node. Falls back on resolving using the linked
    /// Device, if present.
    fn resolve_tag<'a>(
        &'a self,
        state: &'a state::State,
        tag: Tag,
    ) -> Option<&'a String> {
        match tag {
            Tag::Node(NodeTag::NodeName) => self.name.as_ref(),
            Tag::Node(NodeTag::NodeNick) => self.nick.as_ref(),
            Tag::Node(NodeTag::NodeDescription) => self.description.as_ref(),
            Tag::Node(NodeTag::MediaName) => self.media_name.as_ref(),
            Tag::Device(_) => {
                let device = state.devices.get(&self.device_id?)?;
                device.resolve_tag(state, tag)
            }
        }
    }

    fn fallback(&self) -> Option<&String> {
        self.name.as_ref()
    }

    fn templates<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<NameTemplate> {
        match self.media_class.as_ref() {
            Some(media_class)
                if media_class.is_sink() || media_class.is_source() =>
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

/// Internal implementation for name resolution.
///
/// This implements the public [`config::Names::resolve()`] method.
pub fn resolve<T: NameResolver>(
    state: &state::State,
    resolver: &T,
    names: &config::Names,
) -> Option<String> {
    resolver
        .templates(state, names)
        .iter()
        .find_map(|template| {
            template.render(|tag| resolver.resolve_tag(state, *tag))
        })
        .or(resolver.fallback().cloned())
}
