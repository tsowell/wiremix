//! Track nodes being captured.

use std::collections::HashSet;

use crate::command::Command;
use crate::object::ObjectId;
use crate::state::Node;

/// Track nodes being captured. The on_ methods can return a
/// [`Command`](`crate::command::Command`) which should be passed to
/// `crate::monitor::execute_command()` to start or stop capture if needed.
#[derive(Default, Debug)]
pub struct CaptureManager {
    capturing: HashSet<ObjectId>,
}

impl CaptureManager {
    /// Call when a node's capture eligibility might have changed.
    pub fn on_node(&mut self, node: &Node) -> Option<Command> {
        if !node.media_class.as_ref().is_some_and(|media_class| {
            media_class.is_source()
                || media_class.is_sink_input()
                || media_class.is_source_output()
        }) {
            return None;
        }

        node.object_serial?;

        if self.capturing.contains(&node.id) {
            return None;
        }

        self.start_capture_command(node)
    }

    /// Call when a node gets a new input link.
    pub fn on_link(&mut self, node: &Node) -> Option<Command> {
        if !node.media_class.as_ref().is_some_and(|media_class| {
            media_class.is_sink()
                || media_class.is_source()
                || media_class.is_sink_input()
                || media_class.is_source_output()
        }) {
            return None;
        }

        self.start_capture_command(node)
    }

    /// Call when a node's output positions have changed.
    pub fn on_positions_changed(&mut self, node: &Node) -> Option<Command> {
        if !self.capturing.contains(&node.id) {
            return None;
        }

        self.start_capture_command(node)
    }

    /// Call when a node has no more input links.
    pub fn on_removed(&mut self, node: &Node) -> Option<Command> {
        self.stop_capture_command(node)
    }

    fn start_capture_command(&mut self, node: &Node) -> Option<Command> {
        let object_serial = &node.object_serial?;
        let capture_sink =
            node.media_class.as_ref().is_some_and(|media_class| {
                media_class.is_sink() || media_class.is_source()
            });

        self.capturing.insert(node.id);

        Some(Command::NodeCaptureStart(
            node.id,
            *object_serial,
            capture_sink,
        ))
    }

    fn stop_capture_command(&mut self, node: &Node) -> Option<Command> {
        self.capturing.remove(&node.id);

        Some(Command::NodeCaptureStop(node.id))
    }
}
