#[macro_use]
extern crate lanta;

use lanta::layout::*;
use lanta::{cmd, Lanta, ModKey, Result};

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
    lanta::intiailize_logger()?;

    let modkey = ModKey::Mod1;
    let shift = ModKey::Shift;

    #[rustfmt::skip]
    let mut keys = keys![
        ([modkey], XK_d, cmd::lazy::close_focused_window()),
        ([modkey], XK_j, cmd::lazy::focus_next()),
        ([modkey], XK_k, cmd::lazy::focus_previous()),
        ([modkey, shift], XK_j, cmd::lazy::shuffle_next()),
        ([modkey, shift], XK_k, cmd::lazy::shuffle_previous()),
        ([modkey], XK_Tab, cmd::lazy::layout_next()),

        ([modkey], XK_c, spawn!("alacritty")),
        ([modkey], XK_p, spawn!("dmenu_run")),
    ];

    let padding = 0;
    let layouts = layouts![
        StackLayout::new("stack", 0),
        TiledLayout::new("tiled", padding),
    ];

    let groups = groups! {
        keys,
        shift,
        [
            ([modkey], XK_a, "chrome", "stack"),
            ([modkey], XK_s, "code", "stack"),
            ([modkey], XK_d, "term", "tiled"),
            ([modkey], XK_f, "misc", "tiled"),
        ]
    };

    Lanta::new(keys, groups, &layouts)?.run();

    Ok(())
}
