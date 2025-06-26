//! Track nodes being captured.

use std::collections::HashSet;

use crate::command::Command;
use crate::media_class;
use crate::monitor::ObjectId;
use crate::state::Node;

/// Track nodes being captured. This can be passed to
/// `crate::state::State::update()` which uses the on_ methods to queue up
/// start and stop capture commands. Once all updates are complete, the pending
/// commands can be retrieved via `flush()` and executed.
#[derive(Default, Debug)]
pub struct CaptureManager {
    capturing: HashSet<ObjectId>,
    commands: Vec<Command>,
}

impl CaptureManager {
    /// Call when a node's capture eligibility might have changed.
    pub fn on_node(&mut self, node: &Node) {
        if !node
            .props
            .media_class()
            .as_ref()
            .is_some_and(|media_class| {
                media_class::is_source(media_class)
                    || media_class::is_sink_input(media_class)
                    || media_class::is_source_output(media_class)
            })
        {
            return;
        }

        if node.props.object_serial().is_none() {
            return;
        }

        if self.capturing.contains(&node.id) {
            return;
        }

        let command = self.start_capture_command(node);
        self.commands.extend(command);
    }

    /// Call when a node gets a new input link.
    pub fn on_link(&mut self, node: &Node) {
        if !node
            .props
            .media_class()
            .as_ref()
            .is_some_and(|media_class| {
                media_class::is_sink(media_class)
                    || media_class::is_source(media_class)
                    || media_class::is_sink_input(media_class)
                    || media_class::is_source_output(media_class)
            })
        {
            return;
        }

        let command = self.start_capture_command(node);
        self.commands.extend(command);
    }

    /// Call when a node's output positions have changed.
    pub fn on_positions_changed(&mut self, node: &Node) {
        if !self.capturing.contains(&node.id) {
            return;
        }

        let command = self.start_capture_command(node);
        self.commands.extend(command);
    }

    /// Call when a node has no more input links.
    pub fn on_removed(&mut self, node: &Node) {
        let command = self.stop_capture_command(node);
        self.commands.extend(command);
    }

    fn start_capture_command(&mut self, node: &Node) -> Option<Command> {
        let object_serial = *node.props.object_serial()?;
        let capture_sink =
            node.props
                .media_class()
                .as_ref()
                .is_some_and(|media_class| {
                    media_class::is_sink(media_class)
                        || media_class::is_source(media_class)
                });

        self.capturing.insert(node.id);

        Some(Command::NodeCaptureStart(
            node.id,
            object_serial,
            capture_sink,
        ))
    }

    fn stop_capture_command(&mut self, node: &Node) -> Option<Command> {
        self.capturing.remove(&node.id);

        Some(Command::NodeCaptureStop(node.id))
    }

    /// Get a list of pending commands.
    pub fn flush(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }
}
