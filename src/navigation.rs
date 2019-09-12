use crate::viewport::{Viewport};
use crate::layout::MappedWindow;

#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn next_window_line<'a, T: std::fmt::Debug>(
        &self,
        focus: &Viewport,
        windows: &'a Vec<MappedWindow<T>>,
    ) -> Option<&'a MappedWindow<T>> {
        let center_x = focus.x + (focus.width / 2);
        let center_y = focus.y + (focus.height / 2);
        let mut candidates: Vec<_> = windows
            .iter()
            .filter(|w| {
                &w.vp != focus
                    && match self {
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
        candidates.sort_unstable_by_key(|w| match self {
            Direction::Up | Direction::Down => w.vp.y,
            Direction::Left | Direction::Right => w.vp.x,
        });
        debug!("{:?} {:?}", self, candidates);
        match self {
            Direction::Up | Direction::Left => candidates.last(),
            Direction::Down | Direction::Right => candidates.first(),
        }
        .map(|&w| w)
    }
}
#[cfg(test)]
mod direction {
    use super::{Direction, MappedWindow, Viewport};

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
            Direction::Left
                .next_window_line(&windows[2].vp, &windows)
                .unwrap()
                .id,
            1
        );
        assert_eq!(
            Direction::Right
                .next_window_line(&windows[0].vp, &windows)
                .unwrap()
                .id,
            1
        );
    }

    #[test]
    fn vertical_move_picks_the_nearest_candidate() {
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
            Direction::Left
                .next_window_line(&windows[2].vp, &windows)
                .unwrap()
                .id,
            1
        );
        assert_eq!(
            Direction::Right
                .next_window_line(&windows[0].vp, &windows)
                .unwrap()
                .id,
            1
        );
    }
}
