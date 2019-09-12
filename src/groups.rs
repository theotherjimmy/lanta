use std::borrow::Cow;

use crate::layout::Layout;
use crate::x::WindowId;

type LayoutId = usize;

pub struct Group {
    name: Cow<'static, str>,
    pub layout_id: LayoutId,
    pub focused_window: Option<WindowId>,
}

impl Group {
    pub fn new<S, T>(name: S, default_layout: &str, layouts: &[Box<dyn Layout<T>>]) -> Group
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
