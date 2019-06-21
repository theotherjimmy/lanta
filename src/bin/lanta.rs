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
        ([modkey], XK_Right, cmd::lazy::focus_next()),
        ([modkey], XK_Left, cmd::lazy::focus_previous()),
        ([modkey], XK_r, cmd::lazy::next_group()),
        ([modkey], XK_v, cmd::lazy::prev_group()),
        ([modkey, shift], XK_Right, cmd::lazy::shuffle_next()),
        ([modkey, shift], XK_Left, cmd::lazy::shuffle_previous()),
        ([modkey], XK_Tab, cmd::lazy::layout_next()),

        ([modkey], XK_c, spawn!("alacritty")),
        ([modkey], XK_p, spawn!("dmenu_run")),

        ([modkey, shift], XK_a, spawn!("amixer", "-q", "set", "Master", "2%+")),
        ([modkey], XK_a, spawn!("amixer", "-q", "set", "Master", "2%-")),
    ];

    let padding = 2;
    let layouts = layouts![
        StackLayout::new("stack", 0),
        TiledLayout::new("tiled", padding),
        ThreeColumn::new("3 column", padding),
    ];

    let groups = groups! {
        keys,
        shift,
        [
            ([modkey, shift], XK_1, "browser", "stack"),
            ([modkey, shift], XK_3, "term", "3 column"),
            ([modkey, shift], XK_5, "misc", "3 column"),
        ]
    };

    Lanta::new(keys, groups, &layouts)?.run();

    Ok(())
}
