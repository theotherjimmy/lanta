#[macro_use]
extern crate log;

use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
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
use crate::x::{Crtc, CrtcChange, StrutPartial, WindowId, WindowType};

pub use crate::groups::GroupBuilder;
pub use crate::keys::ModKey;
pub use crate::stack::Stack;
pub use crate::x::{Connection, CrtcInfo, Event};

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

#[derive(Clone, Debug)]
struct Strut {
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
    fn from_strut_partial(frm: &StrutPartial) -> Strut {
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    fn clone_from_crtc_info(c: &CrtcInfo) -> Viewport {
        Viewport {
            x: c.x as u32,
            y: c.y as u32,
            width: c.width as u32,
            height: c.height as u32,
        }
    }

    fn without_strut(&self, screen_width: u32, screen_height: u32, strut: &Strut) -> Viewport {
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
            top = cmp::min(top, strut.top)
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

#[cfg(test)]
mod viewport {
    use super::{Strut, Viewport};

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
}

struct Dock {
    window_id: WindowId,
    strut_partial: Option<StrutPartial>,
}

trait Dockable {
    fn get_strut(&self) -> Option<Strut>;
}

impl Dockable for Dock {
    fn get_strut(&self) -> Option<Strut> {
        self.strut_partial.as_ref().map(Strut::from_strut_partial)
    }
}

struct Screen<T> {
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
        debug!("Docks: {:?}", docks);
        let width = ports.iter().map(|v| v.x + v.width).fold(0, cmp::max);
        let height = ports.iter().map(|v| v.y + v.height).fold(0, cmp::max);
        for vp in ports.iter_mut() {
            *vp = docks
                .iter()
                .fold(*vp, |v, s| v.without_strut(width, height, s));
        }
        debug!("Calculated Viewport as {:?}", ports);
        ports
    }
}

#[cfg(test)]
mod screen {
    use super::{Dockable, Screen, Strut, Viewport};

    struct TestDock(Strut);
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
}

type GroupId = usize;

pub struct Lanta {
    connection: Rc<Connection>,
    keys: KeyHandlers,
    groups: Vec<Group>,
    crtc: HashMap<Crtc, (CrtcInfo, GroupId)>,
    screen: Screen<Dock>,
    current_crtc: Crtc,
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

        let groups = groups
            .into_iter()
            .map(|group: GroupBuilder| group.build(connection.clone(), layouts.to_owned()))
            .collect::<Vec<Group>>()
            .into();
        let mut crtc = connection
            .list_crtc()
            .context("Can't start window manager without a crtc map")?;
        crtc.retain(|(_, ci)| ci.width > 0 && ci.height > 0);
        let crtc_len = crtc.len();
        let crtc = crtc
            .into_iter()
            .zip(0..crtc_len)
            .map(|((crtc, info), grp)| (crtc, (info, grp)))
            .collect::<HashMap<_, _>>();
        let current_crtc = crtc.keys().next().unwrap().clone();
        debug!("starting with crtc: {:?}", crtc);
        let mut wm = Lanta {
            keys,
            groups,
            connection: connection,
            screen: Screen::default(),
            crtc,
            children: Vec::new(),
            current_crtc,
        };

        // Learn about existing top-level windows.
        let existing_windows = wm.connection.top_level_windows()?;
        for window in existing_windows {
            wm.manage_window(window);
        }
        wm.activate_current_groups();
        wm.update_ewmh_desktops();

        Ok(wm)
    }

    fn deactivate_all_groups(&mut self) {
        for group in self.groups.iter_mut() {
            group.deactivate();
        }
    }

    fn activate_current_groups(&mut self) {
        let vps = self.viewports();
        let Lanta {
            ref crtc,
            ref mut groups,
            ..
        } = self;
        for ((_info, grp_id), viewport) in crtc.values().zip(vps.into_iter()) {
            if let Some(group) = groups.get_mut(*grp_id) {
                group.activate(viewport);
            }
        }
    }

    fn find_next_unallocated_group(&self) -> GroupId {
        let gidx_set = self
            .crtc
            .values()
            .map(|(_info, gid)| gid.clone())
            .collect::<HashSet<_>>();
        (0..)
            .into_iter()
            .find(|gid| !gidx_set.contains(gid))
            .unwrap()
    }

    fn update_ewmh_desktops(&self) {
        let current_group = self
            .group_idx()
            .expect("Lanta always maintains an active group.");
        self.connection
            .update_ewmh_desktops(&self.groups, current_group)
    }

    fn group_idx(&self) -> Option<usize> {
        self.crtc
            .get(&self.current_crtc)
            .map(|(_info, idx)| idx.clone())
    }

    fn viewports(&mut self) -> Vec<Viewport> {
        let viewports = self
            .crtc
            .values()
            .map(|(info, _)| Viewport::clone_from_crtc_info(info))
            .collect();
        self.screen.viewports(viewports)
    }

    pub fn rotate_crtc(&mut self) {
        let mut iter = self
            .crtc
            .keys()
            .cycle()
            .skip_while(|&crtc_id| &self.current_crtc != crtc_id);
        let _ = iter.next();
        if let Some(next_crtc) = iter.next() {
            self.current_crtc = *next_crtc;
        }
    }

    pub fn wait_on_child(&mut self, cld: Child) {
        self.children.push(cld);
    }

    pub fn group(&self) -> &Group {
        let group_idx = self
            .group_idx()
            .expect("The focused screen must have an active group");
        self.groups
            .get(group_idx)
            .expect("The focused group must exist")
    }

    pub fn group_mut(&mut self) -> &mut Group {
        let group_idx = self
            .group_idx()
            .expect("The focused screen must have an active group");
        self.groups
            .get_mut(group_idx)
            .expect("The focused group must exist")
    }

    fn focus_idx(&mut self, new_idx: usize) {
        if new_idx >= self.groups.len() {
            return;
        }
        let after_insert = self.crtc.get(&self.current_crtc).map(|(_info, gidx)| gidx.clone());
        match after_insert {
            Some(old_idx) if old_idx != new_idx => {
                for (_info, ref mut gid) in self.crtc.values_mut() {
                    if *gid == new_idx {
                        *gid = old_idx;
                    }
                }
            }
            Some(_) | None => (),
        };
        if let Some((_info, idx)) = self.crtc.get_mut(&self.current_crtc) {
            *idx = new_idx;
        }
        self.update_ewmh_desktops();
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

        match self.groups.iter().position(|group| group.name() == name) {
            Some(idx) => {
                self.deactivate_all_groups();
                self.group_mut().deactivate();
                self.focus_idx(idx);
                self.activate_current_groups();
            }
            None => (),
        }
    }

    pub fn next_group(&mut self) {
        self.deactivate_all_groups();
        let current = self.group_idx().unwrap();
        self.focus_idx(current + 1);
        self.activate_current_groups();
    }

    pub fn prev_group(&mut self) {
        info!("prev group");
        self.deactivate_all_groups();
        let current = self.group_idx().unwrap();
        match current.checked_sub(1) {
            Some(new_idx) => {
                self.focus_idx(new_idx);
            }
            None => (),
        };
        self.activate_current_groups();
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
            self.deactivate_all_groups();
            self.next_group();
            self.group_mut().add_window(removed);
            self.activate_current_groups();
        }
    }

    pub fn move_focused_to_prev_group(&mut self) {
        if let Some(removed) = self.group_mut().remove_focused() {
            self.deactivate_all_groups();
            self.prev_group();
            self.group_mut().add_window(removed);
            self.activate_current_groups();
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

        let attrs = self.connection.get_window_attributes(&window_id);
        match attrs {
            Ok(wattrs) => {
                if wattrs.override_redirect() {
                    return;
                }
            }
            Err(e) => {
                warn!("Could not get window attrs for {}: {}", window_id, e);
                return;
            }
        }
        if window_types.contains(&WindowType::Notification)
            || window_types.contains(&WindowType::Tooltip)
            || window_types.contains(&WindowType::Utility)
        {
            return;
        }

        if dock {
            self.connection.map_window(&window_id);
            self.screen.add_dock(&self.connection, window_id);
            self.activate_current_groups();
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
        self.activate_current_groups()
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
                Event::CrtcChange(change) => self.on_crtc_change(&change),
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

    fn on_crtc_change(&mut self, change: &CrtcChange) {
        debug!(
            "Crtc's Changed! Before: {:?}, {:?}",
            &self.crtc, &self.current_crtc
        );
        if change.width > 0 && change.height > 0 {
            let gidx = self.find_next_unallocated_group();
            match self.crtc.entry(change.crtc) {
                Entry::Vacant(v) => {
                    v.insert((change.into(), gidx));
                }
                Entry::Occupied(ref mut o) => {
                    o.get_mut().0 = change.into();
                }
            }
        } else {
            self.crtc.remove(&change.crtc);
            if self.current_crtc == change.crtc {
              self.current_crtc = *self.crtc.keys().next().unwrap();
            }
        }
        debug!(
            "Crtc's Changed! After: {:?}, {:?}",
            &self.crtc, &self.current_crtc
        );
    }
}
