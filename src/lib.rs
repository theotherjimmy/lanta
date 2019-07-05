#![deny(warnings)]

#[macro_use]
extern crate log;

use std::cell::RefCell;
use std::cmp;
use std::process::Child;
use std::rc::Rc;

use failure::{Error, ResultExt};

pub mod cmd;
mod groups;
mod keys;
pub mod layout;
mod stack;
mod x;

use crate::groups::Group;
use crate::keys::{KeyCombo, KeyHandlers};
use crate::layout::Layout;
use crate::x::{Connection, Event, StrutPartial, WindowId, WindowType};

pub use crate::groups::GroupBuilder;
pub use crate::keys::ModKey;
pub use crate::stack::Stack;

pub type Result<T> = std::result::Result<T, Error>;

pub mod keysym {
    pub use x11::keysym::*;
}

/// Initializes a logger using the default configuration.
///
/// Outputs to stdout and `$XDG_DATA/lanta/lanta.log` by default.
/// You should feel free to initialize your own logger, instead of using this.
pub fn intiailize_logger() -> Result<()> {
    log_panics::init();

    let xdg_dirs = xdg::BaseDirectories::with_prefix("lanta")?;
    let log_path = xdg_dirs
        .place_data_file("lanta.log")
        .context("Could not create log file")?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                time::now().rfc3339(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file(&log_path)?)
        .apply()?;

    Ok(())
}

#[macro_export]
macro_rules! keys {
    [ $( ([$( $mod:ident ),*], $key:ident, $cmd:expr) ),+ $(,)*] => (
        vec![
            $( (vec![$( $mod ),*],  $crate::keysym::$key, $cmd) ),+
        ]
    )
}

#[macro_export]
macro_rules! groups {
    {
        $keys:ident,
        $movemodkey:ident,
        [
            $(( [$( $modkey:ident ),+], $key:ident, $name:expr, $layout:expr )),+
            $(,)*
        ]
    }  => {{
        $keys.extend(keys![
            // Switch to group:
            $(
                ([$($modkey),+], $key, $crate::cmd::lazy::switch_group($name))
            ),+,
            // Move window to group:
            $(
                ([$($modkey),+, $movemodkey], $key,  $crate::cmd::lazy::move_window_to_group($name))
            ),+
        ]);
        vec![
            $(
                 $crate::GroupBuilder::new($name, $layout)
            ),+
        ]
    }}
}

#[macro_export]
macro_rules! layouts {
    [$( $layout:expr ),+ $(,)*] => (
        vec![
            $(
                Box::new($layout) as Box<$crate::layout::Layout>
            ),+
        ]
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

struct Dock {
    window_id: WindowId,
    strut_partial: Option<StrutPartial>,
}

#[derive(Default)]
struct Screen {
    vec: RefCell<Vec<Dock>>,
}

impl Screen {
    pub fn add_dock(&mut self, conn: &Connection, window_id: WindowId) {
        let strut_partial = conn.get_strut_partial(&window_id);
        self.vec.borrow_mut().push(Dock {
            window_id,
            strut_partial,
        });
    }

    pub fn remove_dock(&mut self, window_id: &WindowId) {
        self.vec.borrow_mut().retain(|d| &d.window_id != window_id);
    }

    /// Figure out the usable area of the screen based on the STRUT_PARTIAL of
    /// all docks.
    pub fn viewport(&self, screen_width: u32, screen_height: u32) -> Viewport {
        let (left, right, top, bottom) = self
            .vec
            .borrow()
            .iter()
            .filter_map(|o| o.strut_partial.as_ref())
            .fold((0, 0, 0, 0), |(left, right, top, bottom), s| {
                // We don't bother looking at the start/end members of the
                // StrutPartial - treating it more like a Strut.
                (
                    cmp::max(left, s.left()),
                    cmp::max(right, s.right()),
                    cmp::max(top, s.top()),
                    cmp::max(bottom, s.bottom()),
                )
            });
        let viewport = Viewport {
            x: left,
            y: top,
            width: screen_width - left - right,
            height: screen_height - top - bottom,
        };
        debug!("Calculated Viewport as {:?}", viewport);
        viewport
    }
}

pub struct Lanta {
    connection: Rc<Connection>,
    keys: KeyHandlers,
    groups: Stack<Group>,
    screen: Screen,
    children: Vec<Child>,
}

impl Lanta {
    pub fn new<K, G>(keys: K, groups: G, layouts: &[Box<dyn Layout>]) -> Result<Self>
    where
        K: Into<KeyHandlers>,
        G: IntoIterator<Item = GroupBuilder>,
    {
        let keys = keys.into();
        let connection = Rc::new(Connection::connect()?);
        connection.install_as_wm(&keys)?;

        let groups = Stack::from(
            groups
                .into_iter()
                .map(|group: GroupBuilder| group.build(connection.clone(), layouts.to_owned()))
                .collect::<Vec<Group>>(),
        );

        let mut wm = Lanta {
            keys,
            groups,
            connection: connection,
            screen: Screen::default(),
            children: Vec::new(),
        };

        // Learn about existing top-level windows.
        let existing_windows = wm.connection.top_level_windows()?;
        for window in existing_windows {
            wm.manage_window(window);
        }
        let viewport = wm.viewport();
        wm.group_mut().activate(viewport);
        wm.connection.update_ewmh_desktops(&wm.groups);

        Ok(wm)
    }

    fn viewport(&self) -> Viewport {
        let (width, height) = self
            .connection
            .get_window_geometry(self.connection.root_window_id());
        self.screen.viewport(width, height)
    }

    pub fn wait_on_child(&mut self, cld: Child) {
        self.children.push(cld);
    }

    pub fn group(&self) -> &Group {
        self.groups.focused().expect("Invariant: No active group!")
    }

    pub fn group_mut(&mut self) -> &mut Group {
        self.groups
            .focused_mut()
            .expect("Invariant: No active group!")
    }

    pub fn switch_group<'a, S>(&'a mut self, name: S)
    where
        S: Into<&'a str>,
    {
        let name = name.into();

        // If we're already on this group, do nothing.
        if self.group().name() == name {
            return;
        }

        self.group_mut().deactivate();
        self.groups.focus(|group| group.name() == name);
        let viewport = self.viewport();
        self.group_mut().activate(viewport);
        self.connection.update_ewmh_desktops(&self.groups);
    }

    pub fn next_group(&mut self) {
        info!("next group");
        self.group_mut().deactivate();
        self.groups.focus_next();
        let viewport = self.viewport();
        self.group_mut().activate(viewport);
        self.connection.update_ewmh_desktops(&self.groups);
    }

    pub fn prev_group(&mut self) {
        info!("prev group");
        self.group_mut().deactivate();
        self.groups.focus_previous();
        let viewport = self.viewport();
        self.group_mut().activate(viewport);
        self.connection.update_ewmh_desktops(&self.groups);
    }

    /// Move the focused window from the active group to another named group.
    ///
    /// If the other named group does not exist, then the window is
    /// (unfortunately) lost.
    pub fn move_focused_to_group<'a, S>(&'a mut self, name: S)
    where
        S: Into<&'a str>,
    {
        let name = name.into();

        // If the group is currently active, then do nothing. This avoids flicker as we
        // unmap/remap.
        if name == self.group().name() {
            return;
        }

        if let Some(removed) = self.group_mut().remove_focused() {
            let new_group = self.groups.iter_mut().find(|group| group.name() == name);
            match new_group {
                Some(new_group) => {
                    new_group.add_window(removed);
                }
                None => {
                    // It would be nice to put the window back in its group (or avoid taking it out
                    // of its group until we've checked the new group exists), but it's difficult
                    // to do this while keeping the borrow checker happy.
                    error!("Moved window to non-existent group: {}", name);
                }
            }
        }
    }

    pub fn move_focused_to_next_group(&mut self) {
        if let Some(removed) = self.group_mut().remove_focused() {
            self.group_mut().deactivate();
            self.groups.focus_next();
            let new_group = self.groups.focused_mut();
            match new_group {
                Some(new_group) => {
                    new_group.add_window(removed);
                }
                None => {
                    // It would be nice to put the window back in its group (or avoid taking it out
                    // of its group until we've checked the new group exists), but it's difficult
                    // to do this while keeping the borrow checker happy.
                    error!("Moved window {} to non-existent group", removed);
                }
            }
            let viewport = self.viewport();
            self.group_mut().activate(viewport);
        }
    }

    pub fn move_focused_to_prev_group(&mut self) {
        if let Some(removed) = self.group_mut().remove_focused() {
            self.group_mut().deactivate();
            self.groups.focus_previous();
            let new_group = self.groups.focused_mut();
            match new_group {
                Some(new_group) => {
                    new_group.add_window(removed);
                }
                None => {
                    // It would be nice to put the window back in its group (or avoid taking it out
                    // of its group until we've checked the new group exists), but it's difficult
                    // to do this while keeping the borrow checker happy.
                    error!("Moved window {} to non-existent group", removed);
                }
            }
            let viewport = self.viewport();
            self.group_mut().activate(viewport);
        }
    }

    /// Returns whether the window is a member of any group.
    fn is_window_managed(&self, window_id: &WindowId) -> bool {
        self.groups.iter().any(|g| g.contains(window_id))
    }

    pub fn manage_window(&mut self, window_id: WindowId) {
        debug!("Managing window: {}", window_id);

        // If we are already managing the window, then do nothing. We do not
        // want the window to end up in two groups at once. We shouldn't
        // be called in such cases, so treat it as an error.
        if self.is_window_managed(&window_id) {
            error!(
                "Asked to manage window that's already managed: {}",
                window_id
            );
            return;
        }

        let window_types = self.connection.get_window_types(&window_id);

        let dock = window_types.contains(&WindowType::Dock);
        self.connection
            .enable_window_key_events(&window_id, &self.keys);

        if window_types.contains(&WindowType::Notification)
            || window_types.contains(&WindowType::Tooltip)
            || window_types.contains(&WindowType::Utility)
        {
            return;
        }

        if dock {
            self.connection.map_window(&window_id);
            self.screen.add_dock(&self.connection, window_id);
            let viewport = self.viewport();
            self.group_mut().update_viewport(viewport);
        } else {
            self.connection.enable_window_tracking(&window_id);
            self.group_mut().add_window(window_id);
        }
    }

    pub fn unmanage_window(&mut self, window_id: &WindowId) {
        debug!("Unmanaging window: {}", window_id);

        // Remove the window from whichever Group it is in. Special case for
        // docks which aren't in any group.
        self.groups
            .iter_mut()
            .find(|group| group.contains(window_id))
            .map(|group| group.remove_window(window_id));
        self.screen.remove_dock(window_id);

        // The viewport may have changed.
        let viewport = self.viewport();
        self.group_mut().update_viewport(viewport);
    }

    pub fn run(mut self) {
        info!("Started WM, entering event loop.");
        let event_loop_connection = self.connection.clone();
        let event_loop = event_loop_connection.get_event_loop();
        for event in event_loop {
            match event {
                Event::MapRequest(window_id) => self.on_map_request(window_id),
                Event::UnmapNotify(window_id) => self.on_unmap_notify(&window_id),
                Event::DestroyNotify(window_id) => self.on_destroy_notify(&window_id),
                Event::KeyPress(key) => self.on_key_press(key),
                Event::EnterNotify(window_id) => self.on_enter_notify(&window_id),
            }
            self.children = self
                .children
                .into_iter()
                .filter_map(|mut cld| match cld.try_wait() {
                    Ok(Some(status)) => {
                        info!("Reaping child process with exit code {}", status);
                        None
                    }
                    Ok(None) => Some(cld),
                    Err(e) => {
                        warn!("Could not wait on child: {}", e);
                        Some(cld)
                    }
                })
                .collect()
        }
        info!("Event loop exiting");
    }

    fn on_map_request(&mut self, window_id: WindowId) {
        if !self.is_window_managed(&window_id) {
            // If the window isn't in any group, then add it to the current group.
            // (This will have the side-effect of mapping the window, as new windows are focused
            // and focused windows are mapped).
            self.manage_window(window_id);
        } else if self.group().contains(&window_id) {
            // Otherwise, if the window is in the active group, focus it. The application probably
            // wants us to make it prominent. Log as there may be misbehaving applications that
            // constantly re-map windows and cause focus issues.
            info!(
                "Window {} asked to be mapped but is already mapped: focusing.",
                window_id
            );
            self.group_mut().focus(&window_id);
        }
    }

    fn on_unmap_notify(&mut self, window_id: &WindowId) {
        // We only receive an unmap notify event when the window is actually
        // unmapped by its application. When our layouts unmap windows, they
        // (should) do it by disabling event tracking first.
        debug!("ignoring unmap notify request for {}", window_id);
    }

    fn on_destroy_notify(&mut self, window_id: &WindowId) {
        self.unmanage_window(window_id);
    }

    fn on_key_press(&mut self, key: KeyCombo) {
        if let Some(handler) = self.keys.get(&key) {
            if let Err(error) = (handler)(self) {
                error!("Error running command for key command {:?}: {}", key, error);
            }
        }
    }

    fn on_enter_notify(&mut self, window_id: &WindowId) {
        self.group_mut().focus(window_id);
    }
}
