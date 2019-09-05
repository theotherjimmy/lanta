use crate::layout::{Layout, MappedWindow};
use crate::stack::Stack;
use crate::x::WindowId;
use crate::Viewport;

#[derive(Clone)]
pub struct StackLayout {
    name: String,
    padding: u32,
}

impl StackLayout {
    pub fn new<S: Into<String>>(name: S, padding: u32) -> StackLayout {
        StackLayout {
            name: name.into(),
            padding,
        }
    }
}

impl Layout for StackLayout {
    fn name(&self) -> &str {
        &self.name
    }

    fn layout(&self, viewport: &Viewport, stack: &Stack<WindowId>) -> Vec<MappedWindow> {
        match stack.focused() {
            Some(&id) => {
                let vp = Viewport{
                    x: viewport.x + self.padding,
                    y: viewport.y + self.padding,
                    width: viewport.width - (self.padding * 2),
                    height: viewport.height - (self.padding * 2),
                };
                vec![MappedWindow{ vp, id }]
            }
            None => Default::default()
        }
    }
}
