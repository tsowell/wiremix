//! Track nodes being captured.

use std::collections::HashSet;

use crate::media_class;
use crate::monitor::{CommandSender, ObjectId};
use crate::state::Node;

/// Track nodes being captured. This can be passed to
/// `crate::state::State::update()` which uses the on_ methods to issue start
/// and stop capture commands.
pub struct CaptureManager<'a> {
    capturing: HashSet<ObjectId>,
    monitor: &'a dyn CommandSender,
    capture_enabled: bool,
}

impl<'a> CaptureManager<'a> {
    pub fn new(monitor: &'a dyn CommandSender, capture_enabled: bool) -> Self {
        Self {
            capturing: Default::default(),
            monitor,
            capture_enabled,
        }
    }

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

        self.start_capture_command(node);
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

        self.start_capture_command(node);
    }

    /// Call when a node's output positions have changed.
    pub fn on_positions_changed(&mut self, node: &Node) {
        if !self.capturing.contains(&node.id) {
            return;
        }

        self.start_capture_command(node);
    }

    /// Call when a node has no more input links.
    pub fn on_removed(&mut self, node: &Node) {
        self.stop_capture_command(node);
    }

    fn start_capture_command(&mut self, node: &Node) {
        if !self.capture_enabled {
            return;
        }

        let Some(object_serial) = node.props.object_serial() else {
            return;
        };

        let capture_sink =
            node.props
                .media_class()
                .as_ref()
                .is_some_and(|media_class| {
                    media_class::is_sink(media_class)
                        || media_class::is_source(media_class)
                });

        self.capturing.insert(node.id);

        self.monitor
            .node_capture_start(node.id, *object_serial, capture_sink);
    }

    fn stop_capture_command(&mut self, node: &Node) {
        if !self.capture_enabled {
            return;
        }

        self.capturing.remove(&node.id);

        self.monitor.node_capture_stop(node.id);
    }
}
