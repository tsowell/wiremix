//! Media class classification methods.

pub fn is_sink(s: &str) -> bool {
    matches!(s, "Audio/Sink" | "Audio/Duplex")
}

pub fn is_source(s: &str) -> bool {
    matches!(s, "Audio/Source" | "Audio/Duplex" | "Audio/Source/Virtual")
}

pub fn is_sink_input(s: &str) -> bool {
    s == "Stream/Output/Audio"
}

pub fn is_source_output(s: &str) -> bool {
    s == "Stream/Input/Audio"
}
