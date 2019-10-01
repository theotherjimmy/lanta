#[macro_use]
extern crate log;

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::process::Child;
use std::rc::Rc;
use std::error::Error;

pub mod cmd;
mod groups;
mod keys;
pub mod layout;
mod navigation;
mod screen;
mod stack;
mod viewport;
mod x;

use crate::x::{Crtc, CrtcChange, WindowType};
use keys::{KeyCombo, KeyHandlers};
use layout::{Layout, MappedWindow};
use screen::{Dock, Screen};

pub use groups::Group;
pub use keys::ModKey;
pub use navigation::{Center, Direction, Line, NextWindow};
pub use stack::Stack;
pub use viewport::Viewport;
pub use x::{Connection, CrtcInfo, Event, WindowId};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub mod keysym {
    pub use x11::keysym::*;
}

type GroupId = usize;

struct Window {
    id: WindowId,
    group: GroupId,
}

trait InGroup {
    fn in_group(&self, group_id: GroupId) -> Vec<WindowId>;
}

impl InGroup for Vec<Window> {
    fn in_group(&self, group_id: GroupId) -> Vec<WindowId> {
        self.iter()
            .filter(|w| w.group == group_id)
            .map(|w| w.id)
            .collect()
    }
}

pub struct Lanta {
    connection: Rc<Connection>,
    keys: KeyHandlers,
    groups: Vec<Group>,
    windows: Vec<Window>,
    layouts: Vec<Box<dyn Layout<WindowId>>>,
    crtc: HashMap<Crtc, (CrtcInfo, GroupId)>,
    screen: Screen<Dock>,
    current_crtc: Crtc,
    children: Vec<Child>,
    mapped: Vec<MappedWindow<WindowId>>,
}

impl Lanta {
    pub fn new<K, G>(keys: K, groups: G, layouts: Vec<Box<dyn Layout<WindowId>>>) -> Result<Self>
    where
        K: Into<KeyHandlers>,
        G: IntoIterator<Item = Group>,
    {
        let keys = keys.into();
        let connection = Rc::new(Connection::connect()?);
        connection.install_as_wm(&keys)?;

        let groups = groups.into_iter().collect::<Vec<Group>>().into();
        let mut crtc = connection
            .list_crtc()?;
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
            layouts: layouts,
            connection: connection,
            screen: Screen::default(),
            crtc,
            children: Vec::new(),
            current_crtc,
            mapped: Vec::new(),
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

    fn groupref(&self, group_id: GroupId) -> (Stack<WindowId>, &dyn Layout<WindowId>) {
        let windows = self.windows.in_group(group_id);
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
        (windows, layout.as_ref())
    }

    fn activate_current_groups(&mut self) {
        let vps = self.viewports();
        let Lanta { ref crtc, .. } = self;
        let mut new_mapped_windows = Vec::new();
        for ((_crtc_id, (_info, grp_id)), viewport) in crtc.iter().zip(vps.into_iter()) {
            let (windows, layout) = self.groupref(*grp_id);
            new_mapped_windows.extend(layout.layout(&viewport, &windows).into_iter());
        }

        let prev_ids: HashSet<_> = self.mapped.iter().map(|w| w.id).collect();
        let next_ids: HashSet<_> = new_mapped_windows.iter().map(|w| w.id).collect();
        let prev: HashSet<_> = self.mapped.iter().collect();
        let next: HashSet<_> = new_mapped_windows.iter().collect();

        for id in prev_ids.difference(&next_ids) {
            self.connection.disable_window_tracking(id);
            self.connection.unmap_window(id);
            self.connection.enable_window_tracking(id);
        }
        for MappedWindow { id, vp } in next.difference(&prev) {
            self.connection.disable_window_tracking(id);
            self.connection
                .configure_window(id, vp.x, vp.y, vp.width, vp.height);
            self.connection.enable_window_tracking(id);
        }
        for id in next_ids.difference(&prev_ids) {
            self.connection.disable_window_tracking(id);
            self.connection.map_window(id);
            self.connection.enable_window_tracking(id);
        }
        self.mapped = new_mapped_windows;
        self.connection.focus(
            self.crtc
                .get(&self.current_crtc)
                .and_then(|(_info, gid)| self.groups.get(*gid))
                .and_then(|grp| grp.focused_window.as_ref()),
        )
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
        if let Some(window) = self.windows.iter().find(|w| &w.id == id) {
            if let Some(group) = self.groups.get_mut(window.group) {
                if Some(window.id) == group.focused_window {
                    let windows = self.windows.in_group(window.group);
                    group.focused_window = windows
                        .iter()
                        .position(|w| w == id)
                        .and_then(|p| {
                            p.checked_sub(1)
                                .and_then(|p| windows.get(p))
                                .or_else(|| windows.get(p + 1))
                        })
                        .map(|&w| w);
                }
            } else {
                error!(
                    "Removing window {:?} with an invalid group {}",
                    id, window.group
                );
            }
        } else {
            error!("Could not lookup window {:?} to remove", id);
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
                let windows = self.windows.in_group(gid);
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

    pub fn rotate_focus_in_group(&mut self) {
        self.modify_focus_group_window_with(|idx, len| Some(idx.checked_sub(1).unwrap_or(len - 1)));
    }

    pub fn swap_in_direction(&mut self, style: &dyn NextWindow<WindowId>, dir: &Direction) {
        if let Some(&MappedWindow { id, .. }) = self
            .focused_window()
            .and_then(|focused| self.mapped.iter().find(|w| w.id == focused))
            .and_then(|w| style.next_window(dir, &w.vp, &self.mapped))
        {
            self.swap_windows(id, self.focused_window().unwrap())
        }
    }

    pub fn focus_in_direction(&mut self, style: &dyn NextWindow<WindowId>, dir: &Direction) {
        if let Some(&MappedWindow { id, .. }) = self
            .focused_window()
            .and_then(|focused| self.mapped.iter().find(|w| w.id == focused))
            .and_then(|w| style.next_window(dir, &w.vp, &self.mapped))
        {
            self.focus_window(&id)
        }
    }

    fn swap_windows(&mut self, lhs: WindowId, rhs: WindowId) {
        let lhs_pos = self.windows.iter().position(|w| w.id == lhs);
        let rhs_pos = self.windows.iter().position(|w| w.id == rhs);
        match (lhs_pos, rhs_pos) {
            (Some(lhs_pos), Some(rhs_pos)) => {
                self.windows.get_mut(lhs_pos).unwrap().id = rhs;
                self.windows.get_mut(rhs_pos).unwrap().id = lhs;
                self.activate_current_groups();
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
                let windows = self.windows.in_group(gid);
                if let Some(pos) = windows.iter().position(|&w| w == wid) {
                    if let Some(nextpos) = fun(pos, windows.len()) {
                        if let Some(&id) = windows.get(nextpos) {
                            self.swap_windows(wid, id);
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
