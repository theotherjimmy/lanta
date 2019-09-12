use std::cmp;

use crate::x::{CrtcInfo, StrutPartial};

#[derive(Clone, Debug)]
pub struct Strut {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub left_start_y: u32,
    pub left_end_y: u32,
    pub right_start_y: u32,
    pub right_end_y: u32,
    pub top_start_x: u32,
    pub top_end_x: u32,
    pub bottom_start_x: u32,
    pub bottom_end_x: u32,
}

impl Strut {
    pub fn from_strut_partial(frm: &StrutPartial) -> Strut {
        Strut {
            left: frm.left(),
            right: frm.right(),
            top: frm.top(),
            bottom: frm.bottom(),
            left_start_y: frm.left_start_y(),
            left_end_y: frm.left_end_y(),
            right_start_y: frm.right_start_y(),
            right_end_y: frm.right_end_y(),
            top_start_x: frm.top_start_x(),
            top_end_x: frm.top_end_x(),
            bottom_start_x: frm.bottom_start_x(),
            bottom_end_x: frm.bottom_end_x(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn clone_from_crtc_info(c: &CrtcInfo) -> Viewport {
        Viewport {
            x: c.x as u32,
            y: c.y as u32,
            width: c.width as u32,
            height: c.height as u32,
        }
    }

    pub fn without_strut(&self, screen_width: u32, screen_height: u32, strut: &Strut) -> Viewport {
        let mut left = self.x;
        let mut right = self.x + self.width;
        let mut top = self.y;
        let mut bottom = self.y + self.height;
        if (strut.left > 0) && (strut.left_start_y >= top) && (strut.left_end_y <= bottom) {
            left = cmp::max(left, strut.left);
        }
        if (strut.right > 0) && (strut.right_start_y >= top) && (strut.right_end_y <= bottom) {
            right = cmp::min(right, screen_width - strut.right)
        }
        if (strut.top > 0) && (strut.top_start_x >= left) && (strut.top_end_x <= right) {
            top = cmp::max(top, strut.top)
        }
        if (strut.bottom > 0) && (strut.bottom_start_x >= left) && (strut.bottom_end_x <= right) {
            bottom = cmp::min(bottom, screen_height - strut.bottom)
        }
        Viewport {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

#[test]
fn top_strut_within_shrinks() {
    let vp = Viewport {
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };
    let strut = Strut {
        left: 0,
        right: 0,
        top: 35,
        bottom: 0,
        left_start_y: 0,
        left_end_y: 0,
        right_start_y: 0,
        right_end_y: 0,
        top_start_x: 0,
        top_end_x: 2559,
        bottom_start_x: 0,
        bottom_end_x: 0,
    };
    assert_eq!(
        vp.without_strut(2560, 2720, &strut),
        Viewport {
            x: 0,
            y: 35,
            width: 2560,
            height: 1405,
        }
    )
}

#[test]
fn bottom_strut_within_shrinks() {
    let vp = Viewport {
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };
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
    assert_eq!(
        vp.without_strut(2560, 2720, &strut),
        Viewport {
            x: 0,
            y: 0,
            width: 2560,
            height: 1405,
        }
    )
}

#[test]
fn bottom_strut_outside_does_not_change() {
    let vp = Viewport {
        x: 0,
        y: 1440,
        width: 1920,
        height: 1280,
    };
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
    assert_eq!(
        vp.without_strut(2560, 2720, &strut),
        Viewport {
            x: 0,
            y: 1440,
            width: 1920,
            height: 1280,
        }
    )
}
