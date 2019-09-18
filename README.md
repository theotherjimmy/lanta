# lanta

Experiments in creating a tiling X11 window manager in Rust.

Lanta is written to be customisable, simple and fast-ish.

## Features

Lanta doesn't implement all of [EWMH](https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html) or [ICCCM](https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html), nor will it ever. It aims to implement just enough for use as my primary WM.

Lanta tiles windows in groups and each window may be in exactly one group.
Windows can be moved between groups, can be focused inside a group and can be swapped with eathother.
Each group may cycle through a global set of layouts which control how the it's windows are tiled.

There are currently a few simple layouts implemented:

 - Stack — Maximises the currently focused window.
 - Tiled — Tiles all windows in the group's horizontally.
 - 3 column — Balances Windows into 3 columns of windows.

Further, 3 modes of navigation are available:
 - Rotate through the groups windows.
 - Navigate between visible windows on all screens by picking the nearest window that intersects with a ray in a direction.
 - Navigate between visible windows on all screens by picking the nearest window that's center point lies within a cone in a direction.

## Installing

Lanta currently requires the stable version of Rust to compile. 

Your system must first have all of the required [dependencies](#dependencies)

To accept the default key shortcuts, layouts and groups, you can install and run using:

```sh
cargo install lanta
# Run directly or add to your .xinitrc:
lanta
```

## Dependencies

In addition to the Rust dependencies in `Cargo.toml`, Lanta also depends on these system libraries:

 - `x11-xcb`
 - `xcb-util`: `xcb-ewmh` / `xcb-icccm` / `xcb-keysyms`

The following Ubuntu packages should allow your system to meet these requirements:

```sh
sudo apt-get install -y libx11-xcb-dev libxcb-ewmh-dev libxcb-icccm4-dev libxcb-keysyms1-dev
```

## Configuration

The configuration file, `~/.config/lanta/lanta.yaml`, containes a section each for layouts groups and keybindings.


### Layouts

The `layouts` section contains a list of objects that describe a single layout.
A layout is given a name, type, and optional layout-specific attributes, such as padding.

For example, the layouts section of my config file looks like:

```yaml
layouts:
  - name: 3-column
    type: ThreeColumn
    padding: 5
  - name: full screen
    type: Stack
```

### Groups

The `groups` section lists objects that describe groups.
A group object contains a name, and default layout.

For example, my groups section looks like:
```yaml
groups:
  - name: ♅
    layout: 3-column
  - name: ♆
    layout: 3-column
  - name: ♇
    layout: 3-column
  - name: ♈
    layout: 3-column
  - name: ♉
    layout: 3-column
  - name: ♊
    layout: 3-column
```

### Keys

The `keys` section is a map from emacs-like key combination descriptions to actions.
The valid actions are:
 - CloseFocused
 - Focus [Style, Direction]
 - Swap [Style, Direction]
 - GroupNext 
 - MoveToNextGroup
 - GroupPrev 
 - MoveToPrevGroup
 - RotateCrtc
 - RotateLayout
 - RotateFocus
 - Spawn

For example, my keybinding configuration looks like:
```yaml
keys:
  M-d: CloseFocused
  M-l:
    Focus:
      - Center
      - Right
  M-S-l:
    Swap:
      - Center
      - Right
  M-h:
    Focus:
      - Center
      - Left
  M-S-h:
    Swap:
      - Center
      - Left
  M-j:
    Focus:
      - Center
      - Down
  M-S-j:
    Swap:
      - Center
      - Down
  M-k:
    Focus:
      - Center
      - Up
  M-S-k:
    Swap:
      - Center
      - Up
  M-down: GroupNext
  M-S-down: MoveToNextGroup
  M-up: GroupPrev
  M-S-up: MoveToPrevGroup
  M-enter: RotateCrtc
  M-space: RotateLayout
  M-tab: RotateFocus
  M-p:
    Spawn: [rofi, -show, run]
  M-a:
    Spawn: [amixer, -q, set, Master, 2%-]
  M-S-a:
    Spawn: [amixer, -q, set, Master, 2%+]
  M-S-s:
    Spawn: [xset, dpms, force, off]
  M-c:
    Spawn: [alacritty]
```

## License

MIT
