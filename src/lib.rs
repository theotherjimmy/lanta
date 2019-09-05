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

use crate::keys::{KeyCombo, KeyHandlers};
use crate::layout::Layout;
use crate::x::{Crtc, CrtcChange, StrutPartial, WindowId, WindowType};

pub use crate::groups::{Group, GroupRef};
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

struct Window {
    id: WindowId,
    group: GroupId,
}

trait ByGroup {
    fn windows_in_grp(&self, group_id: GroupId) -> Vec<WindowId>;
}

impl ByGroup for Vec<Window> {
    fn windows_in_grp(&self, group_id: GroupId) -> Vec<WindowId> {
        self.iter()
            .filter_map(|w| {
                if w.group == group_id {
                    Some(&w.id)
                } else {
                    None
                }
            })
            .cloned()
            .collect()
    }
}

pub struct Lanta {
    connection: Rc<Connection>,
    keys: KeyHandlers,
    groups: Vec<Group>,
    windows: Vec<Window>,
    layouts: Vec<Box<dyn Layout>>,
    crtc: HashMap<Crtc, (CrtcInfo, GroupId)>,
    screen: Screen<Dock>,
    current_crtc: Crtc,
    children: Vec<Child>,
}

impl Lanta {
    pub fn new<K, G>(keys: K, groups: G, layouts: &[Box<dyn Layout>]) -> Result<Self>
    where
        K: Into<KeyHandlers>,
        G: IntoIterator<Item = Group>,
    {
        let keys = keys.into();
        let connection = Rc::new(Connection::connect()?);
        connection.install_as_wm(&keys)?;

        let groups = groups.into_iter().collect::<Vec<Group>>().into();
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
            windows: Default::default(),
            layouts: layouts.iter().cloned().collect(),
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

    fn activate_current_groups(&self) {
        let vps = self.viewports();
        let Lanta { ref crtc, .. } = self;
        for ((&crtc_id, (_info, grp_id)), viewport) in crtc.iter().zip(vps.into_iter()) {
            let grpref = self.groupref(*grp_id);
            grpref.map_to_viewport(&viewport);
            if crtc_id == self.current_crtc {
                grpref.focus_active_window()
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
            .expect("You have more than MAXINT screens. HOW?")
    }

    fn update_ewmh_desktops(&self) {
        let current_group = self
            .group_idx()
            .expect("Lanta always maintains an active group.");
        self.connection.update_ewmh_desktops(
            &self.groups,
            current_group,
            self.windows.iter().map(|w| &w.id).collect(),
        )
    }

    fn group_idx(&self) -> Option<usize> {
        self.crtc
            .get(&self.current_crtc)
            .map(|(_info, idx)| idx.clone())
    }

    fn viewports(&self) -> Vec<Viewport> {
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
        self.activate_current_groups()
    }

    fn wait_on_child(&mut self, cld: Child) {
        self.children.push(cld);
    }

    fn all_grouprefs<'a>(&'a self) -> Vec<GroupRef<'a>> {
        (0..self.groups.len())
            .map(|gid| self.groupref(gid))
            .collect()
    }

    fn deactivate_all_groups(&mut self) {
        for group in self.all_grouprefs() {
            group.unmap();
        }
    }

    fn groupref<'a>(&'a self, group_id: GroupId) -> GroupRef<'a> {
        let windows = self.windows.windows_in_grp(group_id);
        let group = self
            .groups
            .get(group_id)
            .expect("The focused screen must have an active group");
        let focused_idx = group
            .focused_window
            .and_then(|w_id| windows.iter().position(|&w| w == w_id))
            .unwrap_or_default();
        let windows = Stack::from_parts(windows, focused_idx);
        let layout = self
            .layouts
            .get(group.layout_id)
            .expect("The focused group must have an active layout");
        GroupRef::new(&self.connection, windows, layout.as_ref())
    }

    pub fn group_cycle_layouts(&mut self) {
        let num_layouts = self.layouts.len();
        if let Some(group) = self.group_idx().and_then(|gid| self.groups.get_mut(gid)) {
            if let Some(next_layout) = (group.layout_id + 1).checked_rem(num_layouts) {
                group.layout_id = next_layout;
            }
        }
        self.activate_current_groups()
    }

    pub fn close_focused(&mut self) {
        if let Some(id) = self
            .group_idx()
            .and_then(|gid| self.groups.get(gid))
            .and_then(|g| g.focused_window)
        {
            self.connection.close_window(&id)
        }
    }

    fn add_window_to_active_group(&mut self, id: WindowId) {
        if let Some(gid) = self.group_idx() {
            self.add_window_to_group(id, gid)
        }
    }

    fn add_window_to_group(&mut self, id: WindowId, group: GroupId) {
        self.windows.push(Window { id, group });
        let group = self
            .groups
            .get_mut(group)
            .expect("The focused screen must have an active group");
        if group.focused_window.is_none() {
            group.focused_window = Some(id);
        }
        self.activate_current_groups();
        self.update_ewmh_desktops();
    }

    fn remove_window(&mut self, id: &WindowId) {
        debug!("Removing window {:?}", id);
        if let Some(window) = self.windows.iter().find(|w| &w.id != id) {
            if let Some(group) = self.groups.get_mut(window.group) {
                if Some(window.id) == group.focused_window {
                    debug!("Group old focus: {:?}", group.focused_window);
                    group.focused_window = self
                        .windows
                        .iter()
                        .find(|w| w.group == window.group && &w.id != id)
                        .map(|w| w.id);
                    debug!("Group new focus: {:?}", group.focused_window);
                } else {
                    debug!("Group does not have this window focused");
                }
            } else {
                error!("Removing window that is not in a valid group");
            }
        } else {
            error!("Could not lookup window to remove");
        }
        self.windows.retain(|w| &w.id != id);
        self.activate_current_groups();
        self.update_ewmh_desktops();
    }

    fn focused_window(&self) -> Option<WindowId> {
        self.group_idx()
            .and_then(|gid| self.groups.get(gid))
            .and_then(|g| g.focused_window)
    }

    fn modify_focus_group_window_with(&mut self, fun: impl FnOnce(usize, usize) -> Option<usize>) {
        if let Some(gid) = self.group_idx() {
            if let Some(group) = self.groups.get_mut(gid) {
                let windows = self.windows.windows_in_grp(gid);
                if let Some(new_focus) = group
                    .focused_window
                    .map(|wid| windows.iter().position(|&w| w == wid).unwrap_or_default())
                    .and_then(|w| fun(w, windows.len()))
                {
                    group.focused_window = windows.get(new_focus).cloned();
                    self.activate_current_groups();
                }
            } else {
                error!("Tried to change group focus, but the group_idx is not valid");
            }
        } else {
            error!("Tried to change group focus, but theres is no current group idx");
        }
    }

    pub fn focus_next_in_group(&mut self) {
        self.modify_focus_group_window_with(
            |cur, len| if cur + 1 < len { Some(cur + 1) } else { None },
        );
    }

    pub fn focus_previous_in_group(&mut self) {
        self.modify_focus_group_window_with(|idx, _| idx.checked_sub(1));
    }

    fn swap_windows(&mut self, lhs: WindowId, rhs: WindowId) {
        let lhs_pos = self.windows.iter().position(|w| w.id == lhs);
        let rhs_pos = self.windows.iter().position(|w| w.id == rhs);
        match (lhs_pos, rhs_pos) {
            (Some(lhs_pos), Some(rhs_pos)) => {
                self.windows.get_mut(lhs_pos).unwrap().id = rhs;
                self.windows.get_mut(rhs_pos).unwrap().id = lhs;
            }
            (Some(_), None) => {
                error!("Could not swap; Right window is not present");
            }
            (None, Some(_)) => {
                error!("Could not swap; Left window is not present");
            }
            (None, None) => {
                error!("Could not swap; Both windows are not present");
            }
        }
    }

    fn swap_in_group_with(&mut self, fun: impl FnOnce(usize, usize) -> Option<usize>) {
        if let Some(gid) = self.group_idx() {
            if let Some(wid) = self.groups.get(gid).and_then(|g| g.focused_window) {
                let windows = self.windows.windows_in_grp(gid);
                if let Some(pos) = windows.iter().position(|&w| w == wid) {
                    if let Some(nextpos) = fun(pos, windows.len()) {
                        if let Some(&id) = windows.get(nextpos) {
                            self.swap_windows(wid, id);
                            self.activate_current_groups();
                        }
                    } else {
                        error!("No next position when swapping windows");
                    }
                } else {
                    error!("Could not find current window when swapping windows");
                }
            } else {
                error!("Can't swap without an active window");
            }
        } else {
            error!("Tried to swap, but no group is active");
        }
    }

    pub fn swap_with_next_in_group(&mut self) {
        self.swap_in_group_with(|cur, len| if cur + 1 < len { Some(cur + 1) } else { None })
    }

    pub fn swap_with_previous_in_group(&mut self) {
        self.swap_in_group_with(|cur, _| cur.checked_sub(1))
    }

    fn focus_window(&mut self, wid: &WindowId) {
        if let Some(w) = self.windows.iter().find(|w| &w.id == wid) {
            if let Some(g) = self.groups.get_mut(w.group) {
                g.focused_window = Some(w.id);
                if let Some((&crtc_id, _)) =
                    self.crtc.iter().find(|(_id, (_, gid))| w.group == *gid)
                {
                    self.current_crtc = crtc_id;
                }
                self.activate_current_groups();
            }
        }
    }

    pub fn remove_focused_window(&mut self) {
        if let Some(window_id) = self.focused_window() {
            self.remove_window(&window_id);
            self.activate_current_groups();
        }
    }

    fn move_window_to_group(&mut self, id: WindowId, group: GroupId) {
        for w in &mut self.windows {
            if w.id == id {
                w.group = group
            }
        }
    }

    fn focus_group(&mut self, new_idx: GroupId) {
        if new_idx >= self.groups.len() {
            return;
        }
        let after_insert = self
            .crtc
            .get(&self.current_crtc)
            .map(|(_info, gidx)| gidx.clone());
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

    fn shift_group(&mut self, fun: impl FnOnce(usize, usize) -> Option<usize>) {
        if let Some(next_group) = self.group_idx().and_then(|cur| fun(cur, self.groups.len())) {
            self.deactivate_all_groups();
            self.focus_group(next_group);
            self.activate_current_groups();
        }
    }

    pub fn next_group(&mut self) {
        self.shift_group(|cur, len| if cur + 1 < len { Some(cur + 1) } else { None })
    }

    pub fn prev_group(&mut self) {
        self.shift_group(|cur, _len| cur.checked_sub(1))
    }

    fn move_focused_to_group(&mut self, fun: impl FnOnce(usize, usize) -> Option<usize>) {
        if let Some(gid) = self.group_idx().and_then(|idx| fun(idx, self.groups.len())) {
            if let Some(id) = self.focused_window() {
                self.move_window_to_group(id, gid);
            }
            self.deactivate_all_groups();
            self.focus_group(gid);
            self.activate_current_groups();
        }
    }

    pub fn move_focused_to_next_group(&mut self) {
        self.move_focused_to_group(|cur, len| if cur + 1 < len { Some(cur + 1) } else { None });
    }

    pub fn move_focused_to_prev_group(&mut self) {
        self.move_focused_to_group(|idx, _| idx.checked_sub(1));
    }

    fn is_window_managed(&self, window_id: &WindowId) -> bool {
        self.windows.iter().find(|w| &w.id == window_id).is_some()
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
            self.add_window_to_active_group(window_id);
            self.activate_current_groups();
        }
    }

    pub fn unmanage_window(&mut self, window_id: &WindowId) {
        debug!("Unmanaging window: {}", window_id);
        // Remove the window from whichever Group it is in. Special case for
        // docks which aren't in any group.
        self.screen.remove_dock(window_id);
        if self.is_window_managed(window_id) {
            self.remove_window(window_id);
        }
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
        } else if let Some(w) = self.windows.iter().find(|w| w.id == window_id) {
            if let Some(group) = self.groups.get_mut(w.group) {
                group.focused_window = Some(w.id)
            }
        }
    }

    fn on_unmap_notify(&mut self, window_id: &WindowId) {
        // We only receive an unmap notify event when the window is actually
        // unmapped by its application. When our layouts unmap windows, they
        // (should) do it by disabling event tracking first.
        if self.is_window_managed(window_id) {
            self.remove_window(window_id);
        }
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
        self.focus_window(window_id);
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
                self.current_crtc = *self
                    .crtc
                    .keys()
                    .next()
                    .expect("Must manage at least one screen");
            }
        }
        debug!(
            "Crtc's Changed! After: {:?}, {:?}",
            &self.crtc, &self.current_crtc
        );
    }
}
