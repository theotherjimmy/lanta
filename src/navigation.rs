use crate::layout::MappedWindow;
use crate::viewport::Viewport;

#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub trait NextWindow<T> {
    fn next_window<'a>(
        &self,
        dir: &Direction,
        focus: &Viewport,
        windows: &'a Vec<MappedWindow<T>>,
    ) -> Option<&'a MappedWindow<T>>;
}

pub struct Line();

impl<T> NextWindow<T> for Line {
    fn next_window<'a>(
        &self,
        dir: &Direction,
        focus: &Viewport,
        windows: &'a Vec<MappedWindow<T>>,
    ) -> Option<&'a MappedWindow<T>> {
        let center_x = focus.x + (focus.width / 2);
        let center_y = focus.y + (focus.height / 2);
        let mut candidates: Vec<_> = windows
            .iter()
            .filter(|w| {
                &w.vp != focus
                    && match dir {
                        Direction::Up => {
                            w.vp.y <= center_y
                                && w.vp.x <= center_x
                                && w.vp.x + w.vp.width >= center_x
                        }
                        Direction::Down => {
                            w.vp.y >= center_y
                                && w.vp.x <= center_x
                                && w.vp.x + w.vp.width >= center_x
                        }
                        Direction::Left => {
                            w.vp.x <= center_x
                                && w.vp.y <= center_y
                                && w.vp.y + w.vp.height >= center_y
                        }
                        Direction::Right => {
                            w.vp.x >= center_x
                                && w.vp.y <= center_y
                                && w.vp.y + w.vp.height >= center_y
                        }
                    }
            })
            .collect();
        candidates.sort_unstable_by_key(|w| match dir {
            Direction::Up | Direction::Down => w.vp.y,
            Direction::Left | Direction::Right => w.vp.x,
        });
        match dir {
            Direction::Up | Direction::Left => candidates.last(),
            Direction::Down | Direction::Right => candidates.first(),
        }
        .map(|&w| w)
    }
}

pub struct Center();

impl<T: std::fmt::Debug> NextWindow<T> for Center {
    fn next_window<'a>(
        &self,
        dir: &Direction,
        focus: &Viewport,
        windows: &'a Vec<MappedWindow<T>>,
    ) -> Option<&'a MappedWindow<T>> {
        let center_x = focus.x + (focus.width / 2);
        let center_y = focus.y + (focus.height / 2);
        windows
            .iter()
            .filter_map(|w| {
                let x = w.vp.x + (w.vp.width / 2);
                let y = w.vp.y + (w.vp.height / 2);
                let delta_x: i64 = x as i64 - center_x as i64;
                let delta_y: i64 = y as i64 - center_y as i64;
                // Normalize all movements to pretend to the to the Right
                let (delta_x, delta_y) = match dir {
                    Direction::Right => (delta_x, delta_y),
                    Direction::Left => (-delta_x, delta_y),
                    Direction::Down => (delta_y, delta_x),
                    Direction::Up => (-delta_y, -delta_x),
                };
                if &w.vp != focus && delta_x > 0 && delta_y < delta_x && delta_y >= -delta_x {
                    Some(((delta_x, delta_y), w))
                } else {
                    None
                }
            })
            .min_by(|lhs, rhs| {
                let ((lx, ly), _) = lhs;
                let ((rx, ry), _) = rhs;
                let dist_l = lx.abs() + ly.abs();
                let dist_r = rx.abs() + ry.abs();
                dist_l.cmp(&dist_r).then_with(|| lx.cmp(&rx))
            })
            .map(|(_, &ref w)| w)
    }
}

#[test]
fn horizontal_move_picks_the_nearest_candidate() {
    let windows: Vec<MappedWindow<u32>> = vec![
        MappedWindow {
            id: 0,
            vp: Viewport {
                x: 0,
                y: 35,
                width: 851,
                height: 1405,
            },
        },
        MappedWindow {
            id: 1,
            vp: Viewport {
                x: 854,
                y: 35,
                width: 851,
                height: 1405,
            },
        },
        MappedWindow {
            id: 2,
            vp: Viewport {
                x: 1708,
                y: 35,
                width: 851,
                height: 1405,
            },
        },
    ];
    assert_eq!(
        Line()
            .next_window(&Direction::Left, &windows[2].vp, &windows)
            .unwrap()
            .id,
        1
    );
    assert_eq!(
        Line()
            .next_window(&Direction::Right, &windows[0].vp, &windows)
            .unwrap()
            .id,
        1
    );
    assert_eq!(
        Center()
            .next_window(&Direction::Left, &windows[2].vp, &windows)
            .unwrap()
            .id,
        1
    );
    assert_eq!(
        Center()
            .next_window(&Direction::Right, &windows[0].vp, &windows)
            .unwrap()
            .id,
        1
    );
}
