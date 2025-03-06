//! Format object names using templates from configuration.

use regex::{self, Regex};

use crate::config;
use crate::state;

pub use crate::config::format_string::FormatString;
pub use crate::config::tag::{DeviceTag, NodeTag, Tag};

pub trait NameResolver {
    fn resolve_format_tag<'a>(
        &'a self,
        state: &'a state::State,
        tag: Tag,
    ) -> Option<&'a String>;

    fn fallback(&self) -> Option<&String>;

    fn formats<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<FormatString>;

    fn name_override<'a>(
        &self,
        state: &state::State,
        overrides: &'a [config::NameOverride],
        override_type: config::OverrideType,
    ) -> Option<&'a Vec<FormatString>> {
        overrides.iter().find_map(|name_override| {
            (name_override.types.contains(&override_type)
                && self.resolve_format_tag(state, name_override.property)
                    == Some(&name_override.value))
            .then_some(&name_override.formats)
        })
    }
}

impl NameResolver for state::Device {
    /// Resolve a tag using Device.
    fn resolve_format_tag<'a>(
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

    fn formats<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<FormatString> {
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
    fn resolve_format_tag<'a>(
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
                device.resolve_format_tag(state, tag)
            }
        }
    }

    fn fallback(&self) -> Option<&String> {
        self.name.as_ref()
    }

    fn formats<'a>(
        &self,
        state: &state::State,
        names: &'a config::Names,
    ) -> &'a Vec<FormatString> {
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

/// Tries to resolve a format string.
pub fn try_resolve<T: NameResolver>(
    state: &state::State,
    resolver: &T,
    format: &FormatString,
) -> Option<String> {
    let tags_regex = Regex::new(r"\{([a-z.-:]*)\}").ok()?;

    let format = format.to_string();

    let mut result = format.clone();

    for cap in tags_regex.captures_iter(&format) {
        let tag = cap.get(1)?.as_str();

        let value = resolver.resolve_format_tag(state, tag.parse().ok()?)?;

        let pattern = format!(r"\{{{}\}}", regex::escape(tag));
        let replace_regex = Regex::new(&pattern).ok()?;
        result = replace_regex.replace_all(&result, value).to_string();
    }

    Some(result.to_string())
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
        .formats(state, names)
        .iter()
        .find_map(|format| try_resolve(state, resolver, format))
        .or(resolver.fallback().cloned())
}
