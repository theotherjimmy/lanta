use std::rc::Rc;

use crate::Lanta;
use std::io::Result;

pub type Command = Rc<dyn Fn(&mut Lanta) -> Result<()>>;

/// Lazy-functions which return a `Command` to do the requested action.
// TODO: Consider offering non-lazy versions and then having simple lazy
// wrappers for them.
pub mod lazy {

    use std::process;
    use std::rc::Rc;
    use std::sync::Mutex;

    use super::Command;
    use crate::{Direction, NextWindow, WindowId};

    /// Rotate the active Crtc
    pub fn rotate_crtc() -> Command {
        Rc::new(|ref mut wm| {
            wm.rotate_crtc();
            Ok(())
        })
    }

    /// Closes the currently focused window.
    pub fn close_focused_window() -> Command {
        Rc::new(|ref mut wm| {
            wm.close_focused();
            Ok(())
        })
    }

    pub fn focus_in<S: NextWindow<WindowId> + 'static>(style: S, dir: Direction) -> Command {
        Rc::new(move |ref mut wm| {
            wm.focus_in_direction(&style, &dir);
            Ok(())
        })
    }

    pub fn swap_in<S: NextWindow<WindowId> + 'static>(style: S, dir: Direction) -> Command {
        Rc::new(move |ref mut wm| {
            wm.swap_in_direction(&style, &dir);
            Ok(())
        })
    }

    /// Moves the focus to the previous window in the current group's stack.
    pub fn rotate_focus_in_group() -> Command {
        Rc::new(|ref mut wm| {
            wm.rotate_focus_in_group();
            Ok(())
        })
    }

    /// Cycles to the next layout of the current group.
    pub fn layout_next() -> Command {
        Rc::new(|ref mut wm| {
            wm.group_cycle_layouts();
            Ok(())
        })
    }

    /// Spawns the specified command.
    ///
    /// The returned `Command` will spawn the `Command` each time it is called.
    pub fn spawn(command: process::Command) -> Command {
        let mutex = Mutex::new(command);
        Rc::new(move |ref mut wm| {
            let mut command = mutex.lock().unwrap();
            info!("Spawning: {:?}", *command);
            let child = command.spawn()?;
            wm.wait_on_child(child);
            Ok(())
        })
    }

    pub fn next_group() -> Command {
        Rc::new(|wm| {
            wm.next_group();
            Ok(())
        })
    }

    pub fn prev_group() -> Command {
        Rc::new(|wm| {
            wm.prev_group();
            Ok(())
        })
    }

    /// Moves the focused window on the active group to another group.
    pub fn move_window_to_next_group() -> Command {
        Rc::new(move |wm| {
            wm.move_focused_to_next_group();
            Ok(())
        })
    }

    /// Moves the focused window on the active group to another group.
    pub fn move_window_to_prev_group() -> Command {
        Rc::new(move |wm| {
            wm.move_focused_to_prev_group();
            Ok(())
        })
    }
}
