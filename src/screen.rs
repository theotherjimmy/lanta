use std::cmp;

use crate::viewport::{Strut, Viewport};
use crate::x::{Connection, StrutPartial, WindowId};

pub struct Dock {
    window_id: WindowId,
    strut_partial: Option<StrutPartial>,
}

pub trait Dockable {
    fn get_strut(&self) -> Option<Strut>;
}

impl Dockable for Dock {
    fn get_strut(&self) -> Option<Strut> {
        self.strut_partial.as_ref().map(Strut::from_strut_partial)
    }
}

pub struct Screen<T> {
    docks: Vec<T>,
}

impl<T> Default for Screen<T> {
    fn default() -> Self {
        Self { docks: vec![] }
    }
}

impl Screen<Dock> {
    pub fn add_dock(&mut self, conn: &Connection, window_id: WindowId) {
        let strut_partial = conn.get_strut_partial(&window_id);
        self.add(Dock {
            window_id,
            strut_partial,
        });
    }

    pub fn remove_dock(&mut self, window_id: &WindowId) {
        self.docks.retain(|d| &d.window_id != window_id);
    }
}

impl<T: Dockable> Screen<T> {
    pub fn add(&mut self, dock: T) {
        self.docks.push(dock)
    }
    /// Figure out the usable area of the screen based on the STRUT_PARTIAL of
    /// all docks.
    pub fn viewports(&self, mut ports: Vec<Viewport>) -> Vec<Viewport> {
        let docks: Vec<Strut> = self.docks.iter().filter_map(T::get_strut).collect();
        let width = ports.iter().map(|v| v.x + v.width).fold(0, cmp::max);
        let height = ports.iter().map(|v| v.y + v.height).fold(0, cmp::max);
        for vp in ports.iter_mut() {
            *vp = docks
                .iter()
                .fold(*vp, |v, s| v.without_strut(width, height, s));
        }
        ports
    }
}

#[cfg(test)]
struct TestDock(Strut);
#[cfg(test)]
impl Dockable for TestDock {
    fn get_strut(&self) -> Option<Strut> {
        Some(self.0.clone())
    }
}

#[test]
fn top_dock_only_affects_top_monitor() {
    let vps = vec![
        Viewport {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
        },
        Viewport {
            x: 0,
            y: 1440,
            width: 1920,
            height: 1280,
        },
    ];
    let strut = Strut {
        left: 0,
        right: 0,
        top: 0,
        bottom: 1315,
        left_start_y: 0,
        left_end_y: 0,
        right_start_y: 0,
        right_end_y: 0,
        top_start_x: 0,
        top_end_x: 0,
        bottom_start_x: 0,
        bottom_end_x: 2559,
    };
    let mut screen = Screen::<TestDock>::default();
    screen.add(TestDock(strut));
    assert_eq!(
        screen.viewports(vps),
        vec![
            Viewport {
                x: 0,
                y: 0,
                width: 2560,
                height: 1405,
            },
            Viewport {
                x: 0,
                y: 1440,
                width: 1920,
                height: 1280,
            },
        ]
    )
}
