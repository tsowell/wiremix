# wiremix

wiremix is a TUI audio mixer for PipeWire. Its interface is more or less a
clone of ncpamixer's which was itself inspired by pavucontrol, so users of
either should find it familiar.

## Installation

wiremix depends on Rust and the PipeWire libraries. To install these:

* Ubuntu: `sudo apt install cargo libpipewire-0.3-dev`
* Debian: `sudo apt install cargo libpipewire-0.3-dev`
* Fedora: `sudo dnf install cargo pipewire-devel`
* Arch Linux: `sudo pacman -S rust libpipewire`

## Quick Start

1. Install wiremix with `cargo install wiremix`
2. Run `wiremix` to launch with default settings
3. Use mouse and/or keyboard bindings to operate the mixer
   - Arrow keys/hjkl to navigate and adjust volume
   - Tab/HL to change tabs
   - c to open a dropdown to route audio to a different destination
   - m to mute/unmute
   - d in Input/Output devices to set default source/sink

## Command-line Options

```
PipeWire mixer

Usage: wiremix [OPTIONS]

Options:
  -c, --config <FILE>    Override default config file path
  -r, --remote <NAME>    The name of the remote to connect to
  -f, --fps <FPS>        Target frames per second (or 0 for unlimited)
  -s, --char-set <NAME>  Character set to use
                         [built-in sets: default, compat, extracompat]
  -t, --theme <NAME>     Theme to use
                         [built-in themes: default]
  -p, --peaks <PEAKS>    Audio peak meters [possible values: off, mono, auto]
      --no-mouse         Disable mouse support
      --mouse            Enable mouse support
  -h, --help             Print help
  -V, --version          Print version
```

Command-line options override settings from the configuration file.

## Default Input Bindings

| Input         | Action                  |
| ------------- | ----------------------- |
| q             | Quit                    |
| m             | Toggle mute             |
| d             | Set default source/sink |
| l/Right arrow | Increment volume        |
| h/Left arrow  | Decrement volume        |
| c             | Open dropdown           |
| Esc           | Cancel dropdown         |
| Enter         | Choose dropdown item    |
| j/Down arrow  | Move down               |
| k/Up arrow    | Move up                 |
| H             | Select previous tab     |
| L/Tab         | Select next tab         |
| `             | Set volume 0%           |
| 1             | Set volume 10%          |
| 2             | Set volume 20%          |
| 3             | Set volume 30%          |
| 4             | Set volume 40%          |
| 5             | Set volume 50%          |
| 6             | Set volume 60%          |
| 7             | Set volume 70%          |
| 8             | Set volume 80%          |
| 9             | Set volume 90%          |
| 0             | Set volume 100%         |

Everything except quitting can also be done with the mouse. Some of the
less-intuitive mouse controls are:

* Click the numeric volume percentage to toggle muting.
* Scroll with the mouse wheel or click on scroll buttons (default appearence:
  `•••`) to scroll
* Right-click in the Input/Output Devices tab to set the default source/sink

## Configuration

wiremix can be configured through a TOML configuration file.

The configuration file is searched for in these locations (in order of
precedence):

1. Path specified on the command-line via `-c`/`--config`
2. `$XDG_CONFIG_HOME/wiremix/wiremix.toml`
3. `~/.config/wiremix/wiremix.toml`

This README only describes basic capabilities. Please see
[wiremix.toml](./wiremix.toml) in this repository for detailed documentation on
configuring wiremix. It also provides a reference for wiremix's defaults.

The configuration specified in the file is merged with wiremix's defaults, so
it only needs to specify the options you want to change. It is recommended to
start with an empty configuration file and use this repository's
[wiremix.toml](./wiremix.toml) as a reference.

### Basic Configuration

Everything that can specified on the command-line has a corresponding option in
the configuration file.

```toml
#remote = "pipewire-0"
#fps = 60.0
mouse = true
peaks = "auto"
char_set = "default"
theme = "default"
```

### Keybindings

The configuration file can customize keyboard controls for all wiremix actions.

#### Examples

```toml
keybindings = [
 # Use ncpamixer-style absolute volume bindings
 { key = { Char = "`" }, action = "Nothing" },
 { key = { Char = "0" }, action = { SetAbsoluteVolume = 0.0 } },
 # Chars 1-9 already work like ncpamixer
]
```

```toml
keybindings = [
 # Use F-keys to select tabs
 { key = { F = 1 }, action = { SelectTab = 0 } },
 { key = { F = 2 }, action = { SelectTab = 1 } },
 { key = { F = 3 }, action = { SelectTab = 2 } },
 { key = { F = 4 }, action = { SelectTab = 3 } },
 { key = { F = 5 }, action = { SelectTab = 4 } },
]
```

### Character Sets

Character sets define the symbols used in the user interface. You can define
multiple character sets and switch between them using the `char_set`
configuration option or the `-s`/`--char-set` command-line argument.

There are three built-in character sets.

1. `default` is the default set. It may contain symbols that can't be rendered
   with your terminal or console.
2. `compat` uses only symbols from
   [cross-platform-terminal-characters](https://github.com/ehmicky/cross-platform-terminal-characters).
3. `extracompat` uses only ASCII symbols.

The configuration file allows for both modifying built-in character sets and
creating custom ones.

### Themes

Themes define colors and other text attributes for UI elements. They are
similar to character sets in that you can define your own themes and switch
between them with the `theme` configuration option or the `-t`/`--theme`
command-line arguments.

There are three built-in themes:

1. `default` is the default theme.
2. `nocolor` uses no color, only attributes.
3. `plain` uses only the default style - no colors or attributes.

The configuration file allows for both modifying built-in themes and creating
custom ones.

### Names

You can customize how streams, endpoints, and devices are displayed in the user
interface using a template system to generate names from PipeWire properties.

It's likely that any particular naming scheme won't work well with 100% of your
software and devices, so you can also specify alternate name templates to use
for PipeWire nodes matching configurable criteria.

#### Examples

The defaults are ncpamixer style, but for more compact names you can try:

```toml
[names]
stream = [ "{node:node.name}: {node:media.name}" ]
endpoint = [ "{device:device.nick}", "{node:node.description}" ]
device = [ "{device:device.nick}", "{device:device.description}" ]
```

wiremix's author uses these overrides with the above:

```toml
# Fix for Apple USB-C to 3.5mm adapter whose device.nick is truncated to
# "USB-C to 3.5mm Headphone Jack A".
[[names.overrides]]
types = [ "endpoint", "device" ]
property = "device:device.name"
value = "alsa_card.usb-Apple__Inc._USB-C_to_3.5mm_Headphone_Jack_Adapter_DWH841302FEJKLTA3-00"
templates = [ "{device:device.description}" ]

# Fix for the official Spotify client which has node.name "spotify" and static
# media.name "Spotify", which makes "{node:node.name}: {node:media.name}" a bit
# redundant.
[[names.overrides]]
types = [ "stream" ]
property = "node:node.name"
value = "spotify"
templates = [ "{node:node.name}" ]
```
