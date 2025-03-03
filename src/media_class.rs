#[derive(Debug, Clone, PartialEq)]
pub struct MediaClass(String);

impl From<&str> for MediaClass {
    fn from(s: &str) -> Self {
        MediaClass(s.to_string())
    }
}

impl MediaClass {
    pub fn is_sink(&self) -> bool {
        matches!(self.0.as_str(), "Audio/Sink" | "Audio/Duplex")
    }

    pub fn is_source(&self) -> bool {
        matches!(
            self.0.as_str(),
            "Audio/Source" | "Audio/Duplex" | "Audio/Source/Virtual"
        )
    }

    pub fn is_sink_input(&self) -> bool {
        self.0 == "Stream/Output/Audio"
    }

    pub fn is_source_output(&self) -> bool {
        self.0 == "Stream/Input/Audio"
    }

    pub fn is_monitor(&self) -> bool {
        self.0 == "Audio/Sink"
    }

    pub fn is_recordable(&self) -> bool {
        self.is_source() || self.is_sink() || self.is_sink_input()
    }
}
