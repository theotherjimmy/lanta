use std::cmp::Eq;
use std::fmt;

use crate::stack::Stack;
use crate::Viewport;

mod stack;
mod tiled;

pub use self::stack::StackLayout;
pub use self::tiled::ThreeColumn;
pub use self::tiled::TiledLayout;

#[derive(Debug, Hash)]
pub struct MappedWindow<T> {
    pub id: T,
    pub vp: Viewport,
}

impl<T: PartialEq> PartialEq for MappedWindow<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.id == rhs.id && self.vp == rhs.vp
    }
}

impl<T: Eq> Eq for MappedWindow<T> {}

pub trait Layout<T>: fmt::Debug {
    fn name(&self) -> &str;
    fn layout(&self, viewport: &Viewport, stack: &Stack<T>) -> Vec<MappedWindow<T>>;
}
