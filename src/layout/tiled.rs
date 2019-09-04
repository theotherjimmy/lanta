use crate::layout::{Layout, MappedWindow};
use crate::stack::Stack;
use crate::x::WindowId;
use crate::Viewport;

#[derive(Clone)]
pub struct TiledLayout {
    name: String,
    padding: u32,
}

impl TiledLayout {
    pub fn new<S: Into<String>>(name: S, padding: u32) -> TiledLayout {
        TiledLayout {
            name: name.into(),
            padding,
        }
    }
}

impl Layout for TiledLayout {
    fn name(&self) -> &str {
        &self.name
    }

    fn layout(&self, viewport: &Viewport, stack: &Stack<WindowId>) -> Vec<MappedWindow> {
        let tile_width = ((viewport.width - self.padding) / stack.len() as u32) - self.padding;

        stack
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let vp = Viewport {
                    x: viewport.x + self.padding + (i as u32 * (tile_width + self.padding)),
                    y: viewport.y + self.padding,
                    width: tile_width,
                    height: viewport.height - (self.padding * 2),
                };
                MappedWindow { id, vp }
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct ThreeColumn {
    name: String,
    padding: u32,
}

impl ThreeColumn {
    pub fn new<S: Into<String>>(name: S, padding: u32) -> ThreeColumn {
        ThreeColumn {
            name: name.into(),
            padding,
        }
    }
}

impl Layout for ThreeColumn {
    fn name(&self) -> &str {
        &self.name
    }

    fn layout(&self, viewport: &Viewport, stack: &Stack<WindowId>) -> Vec<MappedWindow> {
        if stack.len() < 3 {
            let tile_width = ((viewport.width - self.padding) / stack.len() as u32) - self.padding;

            stack
                .iter()
                .enumerate()
                .map(|(i, &id)| {
                    let vp = Viewport {
                        x: viewport.x + self.padding + (i as u32 * (tile_width + self.padding)),
                        y: viewport.y + self.padding,
                        width: tile_width,
                        height: viewport.height - (self.padding * 2),
                    };
                    MappedWindow { id, vp }
                })
                .collect()
        } else {
            let tile_width = ((viewport.width - self.padding) / 3) - self.padding;
            let win_per_col = stack.len() / 3;
            let leftovers = stack.len() - 3 * win_per_col;
            let cols = match leftovers {
                2 => vec![
                    stack.slice(0..win_per_col + 1),
                    stack.slice(win_per_col + 1..(2 * win_per_col) + 1),
                    stack.slice((2 * win_per_col) + 1..stack.len()),
                ],
                _ => vec![
                    stack.slice(0..win_per_col),
                    stack.slice(win_per_col..(2 * win_per_col) + leftovers),
                    stack.slice((2 * win_per_col) + leftovers..stack.len()),
                ],
            };
            let mut to_ret = Vec::with_capacity(stack.len());
            for (col, slice) in cols.iter().enumerate() {
                let x = viewport.x + self.padding + (col as u32 * (tile_width + self.padding));
                let tile_height =
                    ((viewport.height - self.padding) / slice.len() as u32) - self.padding;
                for (row, &id) in slice.iter().enumerate() {
                    let vp = Viewport {
                        x,
                        y: viewport.y + self.padding + (row as u32 * (tile_height + self.padding)),
                        width: tile_width,
                        height: tile_height,
                    };
                    to_ret.push(MappedWindow { id, vp });
                }
            }
            to_ret
        }
    }
}
