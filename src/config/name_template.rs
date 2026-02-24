//! A type for validating and rendering name template strings.
//!
//! Templates are strings with tags enclosed in { and }. All tag contents must
//! be parsable into PropertyKeys in order for the string to be accepted.
//! { without a matching } or } without a matching { are invalid.
//! { and } can be escaped with {{ and }}.
use anyhow::{anyhow, bail};
use serde_with::DeserializeFromStr;

use crate::config::property_key::PropertyKey;

#[derive(Debug, DeserializeFromStr)]
#[cfg_attr(test, derive(PartialEq))]
pub struct NameTemplate {
    parts: Vec<Part>,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum Part {
    Literal(String),
    Tag(PropertyKey),
}

impl std::str::FromStr for NameTemplate {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_string(s)
    }
}

impl NameTemplate {
    fn parse_string(s: &str) -> Result<Self, anyhow::Error> {
        // Sort string into literal and tag parts while unescaping {{ and }}
        // to { and }.
        let mut parts = Vec::new();
        let mut chars = s.chars().peekable();
        let mut current_part = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    // Handle escaped brace: {{.
                    if chars.peek() == Some(&'{') {
                        current_part.push('{');
                        chars.next(); // Consume the extra.
                        continue;
                    } else {
                        // Start of a tag.
                        if !current_part.is_empty() {
                            parts.push(Part::Literal(current_part));
                            current_part = String::new();
                        }

                        let tag_content = Self::parse_tag(&mut chars)?;
                        let property_key = tag_content
                            .parse::<PropertyKey>()
                            .map_err(|_| {
                            anyhow!("\"{}\" is not implemented", tag_content)
                        })?;

                        parts.push(Part::Tag(property_key));
                    }
                }
                '}' => {
                    // Handle escaped brace: }}.
                    if chars.peek() == Some(&'}') {
                        current_part.push('}');
                        chars.next(); // Consume the extra.
                    } else {
                        bail!("'}}' without '{{'");
                    }
                }
                _ => current_part.push(ch),
            }
        }

        if !current_part.is_empty() {
            parts.push(Part::Literal(current_part));
        }

        Ok(NameTemplate { parts })
    }

    fn parse_tag(
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<String, anyhow::Error> {
        let mut content = String::new();

        for ch in chars.by_ref() {
            match ch {
                '}' => {
                    return Ok(content);
                }
                '{' => bail!("'{{' without '}}'"),
                _ => content.push(ch),
            }
        }

        Err(anyhow!("'{{' without '}}'"))
    }

    /// Renders a template string using the provided lookup function to convert
    /// Tags into replacement strings.
    pub fn render<T: AsRef<str>>(
        &self,
        lookup: impl Fn(&PropertyKey) -> Option<T>,
    ) -> Option<String> {
        let mut result = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(literal) => result.push_str(literal),
                Part::Tag(property_key) => {
                    result.push_str(lookup(property_key)?.as_ref())
                }
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tags() {
        let s = String::from("Hello");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        assert_eq!(
            template.unwrap(),
            NameTemplate {
                parts: vec![Part::Literal(s.clone())],
            }
        );
    }

    #[test]
    fn good_tag() {
        let s = String::from("Hello {node:node.name}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        assert_eq!(
            template.unwrap(),
            NameTemplate {
                parts: vec![
                    Part::Literal(String::from("Hello ")),
                    Part::Tag(PropertyKey::Node(String::from("node.name"))),
                ],
            }
        );
    }

    #[test]
    fn escapes() {
        let s = String::from("Hello }} {{ {{ {node:node.name} }}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        assert_eq!(
            template.unwrap(),
            NameTemplate {
                parts: vec![
                    Part::Literal(String::from("Hello } { { ")),
                    Part::Tag(PropertyKey::Node(String::from("node.name"))),
                    Part::Literal(String::from(" }")),
                ],
            }
        );
    }

    #[test]
    fn extra_opening() {
        let s = String::from("Hello { {node:node.name}}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_err());
    }

    #[test]
    fn extra_closing() {
        let s = String::from("Hello {node:node.name}}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_err());
    }

    #[test]
    fn empty_tag() {
        let s = String::from("Hello {}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_err());
    }

    #[test]
    fn nested_escapes() {
        let s = String::from("Hello {{{{}}}}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        assert_eq!(
            template.unwrap(),
            NameTemplate {
                parts: vec![Part::Literal(String::from("Hello {{}}")),],
            }
        );
    }

    #[test]
    fn render_empty() {
        let s = String::from("");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        let rendered = template.unwrap().render(|_| None::<&str>);
        assert_eq!(rendered, Some(s));
    }

    #[test]
    fn render_tags() {
        let s = String::from("{node:node.name}{device:device.name}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        let rendered = template.unwrap().render(|tag| match tag {
            PropertyKey::Node(ref s) if s == "node.name" => {
                Some(String::from("foo"))
            }
            PropertyKey::Device(ref s) if s == "device.name" => {
                Some(String::from("bar"))
            }
            _ => None,
        });
        assert_eq!(rendered, Some(String::from("foobar")));
    }

    #[test]
    fn render_missing_tag() {
        let s = String::from("{node:node.name}{device:device.name}");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        let rendered = template.unwrap().render(|tag| match tag {
            PropertyKey::Node(ref s) if s == "node.name" => {
                Some(String::from("foo"))
            }
            _ => None,
        });
        assert_eq!(rendered, None)
    }

    #[test]
    fn render_mixed() {
        let s = String::from("let {node:node.name} = {device:device.name};");
        let template: Result<NameTemplate, _> = s.parse();
        assert!(template.is_ok());
        let rendered = template.unwrap().render(|tag| match tag {
            PropertyKey::Node(ref s) if s == "node.name" => {
                Some(String::from("foo"))
            }
            PropertyKey::Device(ref s) if s == "device.name" => {
                Some(String::from("bar"))
            }
            _ => None,
        });
        assert_eq!(rendered, Some(String::from("let foo = bar;")));
    }
}
