use std::borrow::Cow;
use std::collections::HashSet;

use super::Viewport;
use crate::layout::{Layout, MappedWindow};
use crate::stack::Stack;
use crate::x::{Connection, WindowId};

type LayoutId = usize;

pub struct Group {
    name: Cow<'static, str>,
    pub layout_id: LayoutId,
    pub focused_window: Option<WindowId>,
}

impl Group {
    pub fn new<S>(name: S, default_layout: &str, layouts: &[Box<dyn Layout>]) -> Group
    where
        S: Into<Cow<'static, str>>,
    {
        let layout_id = layouts
            .iter()
            .position(|layout| layout.name() == default_layout)
            .unwrap_or_default();
        Group {
            name: name.into(),
            layout_id,
            focused_window: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct GroupRef<'a> {
    connection: &'a Connection,
    windows: Stack<WindowId>,
    layout: &'a dyn Layout,
}

impl<'a> GroupRef<'a> {
    pub fn new(
        connection: &'a Connection,
        windows: Stack<WindowId>,
        layout: &'a dyn Layout,
    ) -> GroupRef<'a> {
        GroupRef {
            connection,
            windows,
            layout,
        }
    }

    pub fn map_to_viewport(&self, viewport: &Viewport) {
        self.perform_layout(viewport);
    }

    pub fn unmap(&self) {
        for window_id in self.windows.iter() {
            self.connection.disable_window_tracking(window_id);
            self.connection.unmap_window(window_id);
            self.connection.enable_window_tracking(window_id);
        }
    }

    fn perform_layout(&self, vp: &Viewport) {
        let to_map = self.layout.layout(vp, &self.windows);
        let mapped_ids = to_map
            .iter()
            .map(|MappedWindow { id, .. }| id)
            .collect::<HashSet<_>>();
        for id in self.windows.iter() {
            if !mapped_ids.contains(id) {
                self.connection.disable_window_tracking(id);
                self.connection.unmap_window(id);
                self.connection.enable_window_tracking(id);
            }
        }
        for MappedWindow { id, vp } in &to_map {
            self.connection.disable_window_tracking(id);
            self.connection
                .configure_window(id, vp.x, vp.y, vp.width, vp.height);
            self.connection.map_window(id);
            self.connection.enable_window_tracking(id);
        }
    }

    /// Focus the focused window for this group, or to unset the focus when
    /// we have no windows.
    pub fn focus_active_window(&self) {
        match self.windows.focused() {
            Some(window_id) => self.connection.focus_window(window_id),
            None => self.connection.focus_nothing(),
        }
    }
}
