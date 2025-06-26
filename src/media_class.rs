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

pub fn is_monitor(s: &str) -> bool {
    s == "Audio/Sink"
}

pub fn is_recordable(s: &str) -> bool {
    is_source(s) || is_sink(s) || is_sink_input(s)
}

pub fn is_duplex(s: &str) -> bool {
    s == "Audio/Duplex"
}

pub fn is_virtual(s: &str) -> bool {
    s == "Audio/Source/Virtual"
}

pub fn is_stream(s: &str) -> bool {
    is_sink_input(s) || is_source_output(s)
}

pub fn is_device(s: &str) -> bool {
    matches!(
        s,
        "Audio/Sink" | "Audio/Source" | "Audio/Duplex" | "Audio/Source/Virtual"
    )
}
