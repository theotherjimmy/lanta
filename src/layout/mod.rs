use std::fmt;

use crate::stack::Stack;
use crate::Viewport;

mod stack;
mod tiled;

pub use self::stack::StackLayout;
pub use self::tiled::ThreeColumn;
pub use self::tiled::TiledLayout;

#[derive(Debug)]
pub struct MappedWindow<T> {
    pub id: T,
    pub vp: Viewport,
}

pub trait Layout<T>: fmt::Debug {
    fn name(&self) -> &str;
    fn layout(&self, viewport: &Viewport, stack: &Stack<T>) -> Vec<MappedWindow<T>>;
}
