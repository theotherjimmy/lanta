extern crate lanta;

use std::rc::Rc;
use lanta::layout::*;
use lanta::keysym::*;
use lanta::{cmd, Lanta, ModKey, Result, GroupBuilder};

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
        (vec![modkey, shift], XK_Down, cmd::lazy::move_window_to_next_group()),
        (vec![modkey], XK_Up, cmd::lazy::prev_group()),
        (vec![modkey, shift], XK_Up, cmd::lazy::move_window_to_prev_group()),
        (vec![modkey], XK_Tab, cmd::lazy::layout_next()),

        (vec![modkey], XK_c, spawn!("alacritty")),
        (vec![modkey], XK_p, spawn!("dmenu_run")),
        (vec![modkey], XK_a, spawn!("amixer", "-q", "set", "Master", "2%-")),
        (vec![modkey, shift], XK_s, spawn!("slock")),
        (vec![modkey, shift], XK_a, spawn!("amixer", "-q", "set", "Master", "2%+")),
        (vec![modkey], XK_q, Rc::new(|_| panic!("exiting")))
    ];

    let layouts: Vec<Box<dyn Layout>> = vec![
        Box::new(StackLayout::new("stack", 0)),
        Box::new(ThreeColumn::new("3 column", padding)),
    ];
    let groups = vec![
        GroupBuilder::new("browser", "stack"),
        GroupBuilder::new("term", "3 column"),
        GroupBuilder::new("misc", "3 column"),
    ];

    lanta::intiailize_logger()?;
    Lanta::new(keys, groups, &layouts)?.run();

    Ok(())
}
