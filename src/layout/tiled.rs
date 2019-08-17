use crate::layout::Layout;
use crate::stack::Stack;
use crate::x::{Connection, WindowId};
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

    fn layout(&self, connection: &Connection, viewport: &Viewport, stack: &Stack<WindowId>) {
        if stack.is_empty() {
            return;
        }

        let tile_width = ((viewport.width - self.padding) / stack.len() as u32) - self.padding;

        for (i, window_id) in stack.iter().enumerate() {
            connection.disable_window_tracking(window_id);
            connection.map_window(window_id);
            connection.configure_window(
                window_id,
                viewport.x + self.padding + (i as u32 * (tile_width + self.padding)),
                viewport.y + self.padding,
                tile_width,
                viewport.height - (self.padding * 2),
            );
            connection.enable_window_tracking(window_id);
        }
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

    fn layout(&self, connection: &Connection, viewport: &Viewport, stack: &Stack<WindowId>) {
        if stack.is_empty() {
            return;
        }

        if stack.len() < 3 {
            let tile_width = ((viewport.width - self.padding) / stack.len() as u32) - self.padding;

            for (i, window_id) in stack.iter().enumerate() {
                connection.disable_window_tracking(window_id);
                connection.map_window(window_id);
                connection.configure_window(
                    window_id,
                    viewport.x + self.padding + (i as u32 * (tile_width + self.padding)),
                    viewport.y + self.padding,
                    tile_width,
                    viewport.height - (self.padding * 2),
                );
                connection.enable_window_tracking(window_id);
            }
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
            for (col, slice) in cols.iter().enumerate() {
                let x = viewport.x + self.padding + (col as u32 * (tile_width + self.padding));
                let tile_height =
                    ((viewport.height - self.padding) / slice.len() as u32) - self.padding;
                for (row, window_id) in slice.iter().enumerate() {
                    connection.disable_window_tracking(window_id);
                    connection.map_window(window_id);
                    connection.configure_window(
                        window_id,
                        x,
                        viewport.y + self.padding + (row as u32 * (tile_height + self.padding)),
                        tile_width,
                        tile_height,
                    );
                    connection.enable_window_tracking(window_id);
                }
            }
        }
    }
}
