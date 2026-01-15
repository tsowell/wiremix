# wiremix

wiremix is a simple TUI audio mixer for PipeWire. You can use it to adjust
volumes, route audio between devices and applications, and configure audio
device settings like input/output ports and profiles.

wiremix's interface is more or less a clone of the wonderful
[ncpamixer](https://github.com/fulhax/ncpamixer) which was itself inspired by
pavucontrol, so users of either should find it familiar.

Issues and pull requests are welcome!

<img src="https://github.com/user-attachments/assets/26823e34-3a6f-4a3a-bdb2-cde7f3d4cbe5" width="612">

## Installation

### Package Managers

* Arch Linux: Install the [official package](https://archlinux.org/packages/extra/x86_64/wiremix/)
  via `pacman -S wiremix` or `paru -S wiremix-git` for the
  latest development version from the [AUR](https://aur.archlinux.org/packages/wiremix-git).
* Nix: `nix run nixpkgs#wiremix` or add `wiremix` to your configuration.
* Gentoo: Install the [official package](https://packages.gentoo.org/packages/media-sound/wiremix) via
  `emerge -av wiremix`.
* Fedora: Install the [official package](https://src.fedoraproject.org/rpms/rust-wiremix) via
  `dnf in wiremix`.

### Manual Installation

wiremix depends on Rust and the PipeWire libraries. To install all
dependencies:

* Ubuntu: `sudo apt install cargo libpipewire-0.3-dev pkg-config clang`
* Debian: `sudo apt install libpipewire-0.3-dev pkg-config clang` (you will
  also need to install a somewhat recent Rust toolchain - rustup is one way)

Then install wiremix with `cargo install wiremix`

## Quick Start

1. Run `wiremix` to launch with default settings
2. Use mouse and keyboard bindings to operate the mixer
   - ? to display keyboard bindings
   - Arrow keys or hjkl to navigate and adjust volume
   - Tab or HL to change tabs
   - c to open a dropdown to route audio to a different destination
   - m to mute/unmute
   - d set an input or output device as the default source/sink

## Command-line Options

```
PipeWire mixer

Usage: wiremix [OPTIONS]

Options:
  -c, --config <FILE>
          Override default config file path
  -r, --remote <NAME>
          The name of the remote to connect to
  -f, --fps <FPS>
          Target frames per second (or 0 for unlimited)
  -s, --char-set <NAME>
          Character set to use [built-in sets: default, compat, extracompat]
  -t, --theme <NAME>
          Theme to use [built-in themes: default, nocolor, plain]
  -p, --peaks <PEAKS>
          Audio peak meters [possible values: off, mono, auto]
      --no-mouse
          Disable mouse support
      --mouse
          Enable mouse support
  -v, --tab <TAB>
          Initial tab view [possible values: playback, recording, output, input,
          configuration]
  -m, --max-volume-percent <PERCENT>
          Maximum volume for volume sliders
      --no-enforce-max-volume
          Allow increasing volume past max-volume-percent
      --enforce-max-volume
          Prevent increasing volume past max-volume-percent
      --no-lazy-capture
          Monitor peak levels of all nodes
      --lazy-capture
          Only monitor peak levels of on-screen nodes (reduces CPU usage, but
          peaks appear with a slight delay)
  -h, --help
          Print help
  -V, --version
          Print version
```

Command-line options override corresponding settings in the configuration file.

## Input Bindings

Everything except quitting can also be done with the mouse. Some of the
less-intuitive mouse controls are:

* Click the numeric volume percentage to toggle muting.
* Scroll through lists and dropdowns with the mouse wheel or click on scroll
  buttons (default appearance: `•••`)
* Right-click to set as the default source/sink

### Default Keyboard Bindings

| Input         | Action                  |
| ------------- | ----------------------- |
| q             | Quit                    |
| m             | Toggle mute             |
| d             | Set default source/sink |
| l/Right arrow | Increment volume        |
| h/Left arrow  | Decrement volume        |
| Enter/c       | Open dropdown or choose |
| Esc           | Cancel dropdown         |
| j/Down arrow  | Move down               |
| k/Up arrow    | Move up                 |
| H/Shift+Tab   | Select previous tab     |
| L/Tab         | Select next tab         |
| ` (Backtick)  | Set volume 0%           |
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
| ?             | Toggle help screen      |

## Configuration

wiremix can be configured through a TOML configuration file.

It searches for the configuration file in these locations (in order of
precedence):

1. Path specified on the command-line via `-c`/`--config`
2. `$XDG_CONFIG_HOME/wiremix/wiremix.toml`
3. `~/.config/wiremix/wiremix.toml`

This README only describes basic capabilities. Please see
[wiremix.toml](./wiremix.toml) in this repository for detailed documentation on
configuring wiremix. It also provides a reference for all of wiremix's
defaults.

The configuration specified in the file is merged with wiremix's defaults, so
it only needs to specify the options that need to be changed. It is recommended
to start with an empty configuration file and use this repository's
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
tab = "playback"
max_volume_percent = 150.0
enforce_max_volume = false
lazy_capture = false
```

### Keybindings

The configuration file can customize keyboard controls for all wiremix actions.
See [wiremix.toml](./wiremix.toml) for more details.

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

See [wiremix.toml](./wiremix.toml) for more details.

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

See [wiremix.toml](./wiremix.toml) for more details.

### Names

You can customize how streams, endpoints, and devices are displayed in the user
interface using a template system to generate names from PipeWire properties.

It's likely that any particular naming scheme won't work well with 100% of your
software and devices, so you can also specify alternate name templates to use
for PipeWire nodes matching configurable criteria.

See [wiremix.toml](./wiremix.toml) for more details.

#### Examples

The default naming scheme is:

```toml
[names]
stream = [ "{node:node.name}: {node:media.name}" ]
endpoint = [ "{device:device.nick}", "{node:node.description}" ]
device = [ "{device:device.nick}", "{device:device.description}" ]
```

Not all nodes and devices have the same properties present, so if multiple
naming templates are specified, wiremix will try to resolve them in order and
use the first one that works.

For ncpamixer-style names you can use:

```toml
[names]
stream = [ "{node:node.name}: {node:media.name}" ]
endpoint = [ "{node:node.description}" ]
device = [ "{device:device.description}" ]
```

I use these overrides with the default names:

```toml
# This device's device.name is truncated to "USB-C to 3.5mm Headphone Jack
# A". This override makes wiremix use device.description instead, which for
# this device is "USB-C to 3.5mm Headphone Jack Adapter".
[[names.overrides]]
types = [ "endpoint", "device" ]
property = "device:device.name"
value = "alsa_card.usb-Apple__Inc._USB-C_to_3.5mm_Headphone_Jack_Adapter_DWH841302FEJKLTA3-00"
templates = [ "{device:device.description}" ]

# The Spotify client's node.name is "spotify", and it also uses "Spotify" for
# media.name. This override makes wiremix use just the node.name, so it shows
# as "spotify" instead of "spotify: Spotify".
[[names.overrides]]
types = [ "stream" ]
property = "node:node.name"
value = "spotify"
templates = [ "{node:node.name}" ]

# mpv is also a bit redundant with the default naming scheme - it suffixes
# media.name with "- mpv". This override makes it show as "foo - mpv" instead
# of "mpv: foo - mpv".
[[names.overrides]]
types = [ "stream" ]
property = "node:node.name"
value = "mpv"
templates = [ "{node:media.name}" ]
```
