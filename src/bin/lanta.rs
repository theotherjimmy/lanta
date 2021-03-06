use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::raw::c_uint;
use std::str::{from_utf8, FromStr};

use directories::ProjectDirs;

use serde::de;
use serde::{Deserialize, Deserializer};

use lanta::keysym::*;
use lanta::layout::*;
use lanta::{
    cmd, Center, Direction, Group, Lanta, Line, ModKey, NextWindow, Result as LantaResult,
    Viewport, WindowId,
};

#[derive(Deserialize, Debug)]
enum Dir {
    Left,
    Right,
    Up,
    Down,
}
impl Into<Direction> for Dir {
    fn into(self) -> Direction {
        match self {
            Dir::Left => Direction::Left,
            Dir::Right => Direction::Right,
            Dir::Up => Direction::Up,
            Dir::Down => Direction::Down,
        }
    }
}

#[derive(Deserialize, Debug)]
enum Style {
    Line,
    Center,
}
enum NextWindowStyle {
    Line(Line),
    Center(Center),
}

impl Into<NextWindowStyle> for Style {
    fn into(self) -> NextWindowStyle {
        match self {
            Style::Line => NextWindowStyle::Line(Line()),
            Style::Center => NextWindowStyle::Center(Center()),
        }
    }
}

impl NextWindow<WindowId> for NextWindowStyle {
    fn next_window<'a>(
        &self,
        dir: &Direction,
        vp: &Viewport,
        wins: &'a Vec<MappedWindow<WindowId>>,
    ) -> Option<&'a MappedWindow<WindowId>> {
        match self {
            NextWindowStyle::Line(l) => l.next_window(dir, vp, wins),
            NextWindowStyle::Center(c) => c.next_window(dir, vp, wins),
        }
    }
}

#[derive(Deserialize, Debug)]
enum Command {
    CloseFocused,
    Focus(Style, Dir),
    Swap(Style, Dir),
    GroupNext,
    GroupPrev,
    MoveToNextGroup,
    MoveToPrevGroup,
    RotateCrtc,
    RotateLayout,
    RotateFocus,
    Spawn(Vec<String>),
}

impl Into<cmd::Command> for Command {
    fn into(self) -> cmd::Command {
        match self {
            Command::CloseFocused => cmd::lazy::close_focused_window(),
            Command::Focus(s, d) => {
                let style: NextWindowStyle = s.into();
                cmd::lazy::focus_in(style, d.into())
            }
            Command::Swap(s, d) => {
                let style: NextWindowStyle = s.into();
                cmd::lazy::swap_in(style, d.into())
            }
            Command::GroupNext => cmd::lazy::next_group(),
            Command::GroupPrev => cmd::lazy::prev_group(),
            Command::MoveToNextGroup => cmd::lazy::move_window_to_next_group(),
            Command::MoveToPrevGroup => cmd::lazy::move_window_to_prev_group(),
            Command::RotateCrtc => cmd::lazy::rotate_crtc(),
            Command::RotateLayout => cmd::lazy::layout_next(),
            Command::RotateFocus => cmd::lazy::rotate_focus_in_group(),
            Command::Spawn(cmd) => {
                let mut command = std::process::Command::new(&cmd[0]);
                command.args(&cmd[1..]);
                cmd::lazy::spawn(command)
            }
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug)]
struct KeyInner {
    mods: Vec<ModKey>,
    key: c_uint,
}

impl FromStr for KeyInner {
    type Err = String;
    fn from_str(frm: &str) -> Result<Self, String> {
        let mut iter = frm.rsplit("-");
        let key = match iter.next().ok_or(String::from("no key found"))? {
            "a" => XK_a,
            "b" => XK_b,
            "c" => XK_c,
            "d" => XK_d,
            "e" => XK_e,
            "f" => XK_f,
            "g" => XK_g,
            "h" => XK_h,
            "i" => XK_i,
            "j" => XK_j,
            "k" => XK_k,
            "l" => XK_l,
            "m" => XK_m,
            "n" => XK_n,
            "o" => XK_o,
            "p" => XK_p,
            "q" => XK_q,
            "r" => XK_r,
            "s" => XK_s,
            "t" => XK_t,
            "u" => XK_u,
            "v" => XK_v,
            "w" => XK_w,
            "x" => XK_x,
            "y" => XK_y,
            "z" => XK_z,
            "space" => XK_space,
            "enter" => XK_Return,
            "tab" => XK_Tab,
            "down" => XK_Down,
            "up" => XK_Up,
            a => Err(format!("Could not match key {}", a))?,
        };
        let mods = iter
            .map(|mod_key| match mod_key {
                "C" => Ok(ModKey::Control),
                "M" => Ok(ModKey::Mod1),
                "S" => Ok(ModKey::Shift),
                a => Err(format!("Did not understand modifier {}", a)),
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(KeyInner { mods, key })
    }
}

impl<'de> Deserialize<'de> for KeyInner {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum LayoutSelectInner {
    ThreeColumn {
        #[serde(default)]
        padding: u32,
    },
    Stack {
        #[serde(default)]
        padding: u32,
    },
}

#[derive(Deserialize, Debug)]
struct LayoutSelect {
    name: String,
    #[serde(flatten)]
    layout: LayoutSelectInner,
}

impl Into<Box<dyn Layout<WindowId>>> for LayoutSelect {
    fn into(self) -> Box<dyn Layout<WindowId>> {
        match self.layout {
            LayoutSelectInner::ThreeColumn { padding } => {
                Box::new(ThreeColumn::new(self.name, padding))
            }
            LayoutSelectInner::Stack { padding } => Box::new(StackLayout::new(self.name, padding)),
        }
    }
}

#[derive(Deserialize, Debug)]
struct GroupDesc {
    name: String,
    layout: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    keys: HashMap<KeyInner, Command>,
    layouts: Vec<LayoutSelect>,
    groups: Vec<GroupDesc>,
}

#[derive(Debug)]
struct NoProjectDir {}
impl std::fmt::Display for NoProjectDir {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, "Cound not find project dirs")
    }
}
impl std::error::Error for NoProjectDir {}

fn main() -> LantaResult<()> {
    env_logger::init();
    let dirs = ProjectDirs::from("org", "foo", "lanta").ok_or(NoProjectDir {})?;
    let mut config_path = dirs.config_dir().to_path_buf();
    config_path.push("lanta.yaml");
    let mut config_file = File::open(config_path)?;
    let mut buffer = Vec::new();
    config_file.read_to_end(&mut buffer)?;
    let Config {
        keys,
        layouts,
        groups,
    } = serde_yaml::from_str(from_utf8(&buffer).unwrap())?;
    let keys: Vec<_> = keys
        .into_iter()
        .map(|(k, v)| (k.mods, k.key, v.into()))
        .collect();
    let layouts: Vec<_> = layouts.into_iter().map(|l| l.into()).collect();
    let groups: Vec<_> = groups
        .into_iter()
        .map(|g| Group::new(g.name, &g.layout, &layouts))
        .collect();

    Lanta::new(keys, groups, layouts)?.run();

    Ok(())
}
