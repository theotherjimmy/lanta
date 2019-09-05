extern crate lanta;

use lanta::keysym::*;
use lanta::layout::*;
use lanta::{cmd, Group, Lanta, ModKey, Result};
use std::rc::Rc;

macro_rules! spawn {
    ($cmd:expr) => (::lanta::cmd::lazy::spawn(::std::process::Command::new($cmd)));
    ($cmd:expr, $($arg:expr),*) => {{
        let mut command = ::std::process::Command::new($cmd);
        $(
            command.arg($arg);
        )*
        ::lanta::cmd::lazy::spawn(command)
    }}
}

fn main() -> Result<()> {
    let padding = 2;
    let modkey = ModKey::Mod1;
    let shift = ModKey::Shift;

    let keys = vec![
        (vec![modkey], XK_d, cmd::lazy::close_focused_window()),
        (vec![modkey], XK_l, cmd::lazy::focus_next()),
        (vec![modkey, shift], XK_l, cmd::lazy::shuffle_next()),
        (vec![modkey], XK_h, cmd::lazy::focus_previous()),
        (vec![modkey, shift], XK_h, cmd::lazy::shuffle_previous()),
        (vec![modkey], XK_Down, cmd::lazy::next_group()),
        (vec![modkey], XK_Return, cmd::lazy::rotate_crtc()),
        (
            vec![modkey, shift],
            XK_Down,
            cmd::lazy::move_window_to_next_group(),
        ),
        (vec![modkey], XK_Up, cmd::lazy::prev_group()),
        (
            vec![modkey, shift],
            XK_Up,
            cmd::lazy::move_window_to_prev_group(),
        ),
        (vec![modkey], XK_Tab, cmd::lazy::layout_next()),
        (vec![modkey], XK_c, spawn!("alacritty")),
        (vec![modkey], XK_p, spawn!("rofi", "-show", "run")),
        (
            vec![modkey],
            XK_a,
            spawn!("amixer", "-q", "set", "Master", "2%-"),
        ),
        (
            vec![modkey, shift],
            XK_s,
            spawn!("xset", "dpms", "force", "off"),
        ),
        (
            vec![modkey, shift],
            XK_a,
            spawn!("amixer", "-q", "set", "Master", "2%+"),
        ),
        (vec![modkey], XK_q, Rc::new(|_| panic!("exiting"))),
    ];

    let layouts: Vec<Box<dyn Layout>> = vec![
        Box::new(StackLayout::new("stack", 0)),
        Box::new(ThreeColumn::new("3 column", padding)),
    ];
    let groups = vec![
        Group::new("\u{2645}", "stack", &layouts ),
        Group::new("\u{2646}", "3 column", &layouts ),
        Group::new("\u{2647}", "3 column", &layouts ),
        Group::new("\u{2648}", "3 column", &layouts ),
        Group::new("\u{2649}", "3 column", &layouts ),
        Group::new("\u{264a}", "3 column", &layouts ),
    ];

    lanta::intiailize_logger()?;
    Lanta::new(keys, groups, &layouts)?.run();

    Ok(())
}
