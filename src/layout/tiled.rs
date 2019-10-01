use crate::layout::{Layout, MappedWindow};
use crate::stack::Stack;
use crate::Viewport;

#[derive(Debug)]
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

impl<T: Copy> Layout<T> for TiledLayout {
    fn name(&self) -> &str {
        &self.name
    }

    fn layout(&self, viewport: &Viewport, stack: &Stack<T>) -> Vec<MappedWindow<T>> {
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

#[derive(Debug)]
pub struct ThreeColumn {
    name: String,
    inner_padding: u32,
}

impl ThreeColumn {
    pub fn new<S: Into<String>>(name: S, inner_padding: u32) -> ThreeColumn {
        ThreeColumn {
            name: name.into(),
            inner_padding,
        }
    }
}

impl<T: Copy> Layout<T> for ThreeColumn {
    fn name(&self) -> &str {
        &self.name
    }

    fn layout(&self, viewport: &Viewport, stack: &Stack<T>) -> Vec<MappedWindow<T>> {
        match stack.len() {
            0 => Default::default(),
            1 => vec![MappedWindow {
                id: stack.iter().next().unwrap().clone(),
                vp: viewport.clone(),
            }],
            2 => {
                let left_width = (viewport.width - self.inner_padding) / 3;
                let right_width = 2 * left_width
                    + (viewport.width - self.inner_padding)
                        .checked_rem(3)
                        .unwrap();
                let viewports = vec![
                    Viewport {
                        x: viewport.x,
                        y: viewport.y,
                        width: left_width,
                        height: viewport.height,
                    },
                    Viewport {
                        x: viewport.x + self.inner_padding + left_width,
                        y: viewport.y,
                        width: right_width,
                        height: viewport.height,
                    },
                ];

                stack
                    .iter()
                    .zip(viewports.into_iter())
                    .map(|(&id, vp)| MappedWindow { id, vp })
                    .collect()
            }
            _ => {
                let tile_width = (viewport.width - (self.inner_padding * 2)) / 3;
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
                    let x = viewport.x + (col as u32 * (tile_width + self.inner_padding));
                    let tile_height = (viewport.height
                        - (self.inner_padding * (slice.len() as u32 - 1)))
                        / slice.len() as u32;
                    for (row, &id) in slice.iter().enumerate() {
                        let vp = Viewport {
                            x,
                            y: viewport.y + (row as u32 * (tile_height + self.inner_padding)),
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
}
