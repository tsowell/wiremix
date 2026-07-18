# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

wiremix is a **dual-mode** PipeWire mixer written in Rust: an interactive
ratatui **TUI** (modeled on `ncpamixer`/`pavucontrol`) plus a machine-readable,
agent-native **CLI**. There is also an **optional Slint desktop GUI**
(`wiremix-gui`, behind the `gui` cargo feature; see "The desktop GUI" below) —
a third frontend over the same core, off by default. It is a **Spacecraft
Software umbrella project** (`GPL-3.0-or-later`) — a fork of upstream `wiremix`
by Thomas Sowell (originally `MIT OR Apache-2.0`,
<https://github.com/tsowell/wiremix>; see `CREDITS.md`).

**The Steelbore Standard, the CLI Standard, and the Agentic CLI layer
apply.** Load the `spacecraft-standard`, `spacecraft-cli-standard`,
`spacecraft-agentic-cli`, and `microsoft-rust-guidelines` skills before
substantive work. Key invariants: REUSE/SPDX two-tag header on every file
(`reuse lint` must pass); new code is `GPL-3.0-or-later` with Mohamed Hammad
copyright, but **don't strip upstream `MIT OR Apache-2.0` headers** from files
you don't substantially rewrite; signed/verified commits (§6.3). The §2 naming
convention is met by a documented carve-out — the upstream name `wiremix` is
deliberately kept (§5.4).

## Build, test, lint

A C toolchain and PipeWire headers are required (the `pipewire`/`libspa` crates
use bindgen). **Build inside `nix develop`** — it provides `pkg-config`, the
PipeWire libs, and the bindgen hook; outside it the build fails at `libspa-sys`.
MSRV is **1.74.0** for the default TUI/CLI; the optional `gui` feature (Slint)
needs **Rust ≥ 1.88**, so `--all-features` (below) requires the newer toolchain
the dev shell ships.

```sh
nix develop                  # interactive dev shell (.envrc autoloads via direnv); or prefix one-shot:

nix develop -c cargo build  --locked --all-features --all-targets   # --all-features includes `gui` (Slint)
nix develop -c cargo build  --locked --features gui --bin wiremix-gui # just the optional desktop GUI
nix develop -c cargo test   --locked --all-features --all-targets
nix develop -c cargo test   --locked --all-features --doc        # doc tests, run separately in CI
nix develop -c cargo test   <name>                               # single test, e.g. `cargo test volume_limit_at_max`

nix develop -c cargo fmt --all --check                           # rustfmt.toml sets max_width = 80
nix develop -c cargo clippy --locked --all-features --all-targets -- -D warnings
nix develop -c reuse lint                                        # REUSE/SPDX must pass
RUSTDOCFLAGS=-D warnings nix develop -c cargo doc --locked --no-deps --document-private-items
```

CI (`.github/workflows/ci.yml`) runs all of the above plus `reuse lint` and
`nixfmt -sw 80 --check` on `*.nix`. Clippy, doc, and reuse failures are hard
errors — keep them clean.

Tests are inline `#[cfg(test)] mod tests` blocks (see `src/app.rs`); there is no
`tests/` directory. The CLI itself can't be tested headless — `App` is tested
against a mock `CommandSender` (`src/lib.rs` `mod mock`) fed synthetic
`StateEvent`s.

### Debugging the PipeWire layer

- `cargo run -- --dump-events` — debug-builds-only flag (`#[cfg(debug_assertions)]`)
  that prints raw PipeWire events instead of starting the UI. The first tool for
  diagnosing monitor/state bugs.
- Debug builds **exit** on any PipeWire error (so you see it); release builds
  silently ignore them (`PipewireError::handle` in `src/app.rs`).
- `--features trace` enables `tracing` logging to `./wiremix.log`, gated on
  `RUST_LOG` (see `src/trace.rs`, `trace_dbg!` macro).

## Architecture

### Threads and channels

Three threads, all wired through `std::sync::mpsc`, set up in `src/main.rs`:

1. **UI thread** — `app::App::run`, the ratatui render + event loop. Owns the
   single `Receiver<event::Event>`.
2. **PipeWire thread** — `wirehose::Session::spawn` runs the PipeWire `MainLoop`.
   Pushes `Event::Pipewire` onto the channel.
3. **Input thread** — `input::spawn`, an async crossterm `EventStream`. Pushes
   `Event::Input`.

`event::Event` is the union (`Input(crossterm) | Pipewire(wirehose)`). Commands
flow the *other* way: `App` holds `&dyn CommandSender` (the `Session`) and sends
`Command`s over a `pipewire::channel` to be run on the PipeWire thread by
`wirehose::execute::execute_command`. Both worker threads are joined on drop of
their handles (`Session`/`InputHandle`); PipeWire shutdown is signaled via an
`EventFd`.

### The `wirehose` module is the only thing that touches PipeWire

`src/wirehose/` is a self-contained, event-based wrapper around `pipewire-rs`.
Everything else in the crate is PipeWire-agnostic and talks to it only through
its public surface (`src/wirehose.rs`): `Session`, `Event`/`StateEvent`/
`PipewireError`, the `CommandSender` trait (exists so the UI can be mocked),
`ObjectId`, `PropertyStore`, `state::State`, `PeakProcessor`.

Internals: per-object-type monitors (`node`, `device`, `link`, `metadata`,
`client`) register listeners on registry globals. Proxies/streams must outlive
their callbacks and **cannot be dropped from inside a PipeWire callback**, so
`proxy_registry`/`stream_registry` defer destruction and trigger collection via
an `EventFd` (`collect_garbage`). `sync_registry` tracks core `done` sequences
to know when the initial enumeration is complete (→ `Event::Ready`).

### State → View → ObjectList → widgets (the render pipeline)

PipeWire data is transformed through three layers, each further from PipeWire
and closer to the screen:

1. `wirehose::state::State` — authoritative model (`nodes`, `devices`, `links`,
   `metadata`, `clients`), mutated only by applying `StateEvent`s.
2. `view::View` — a render-friendly projection built from `State` + config
   `names` + `filters`. Rebuilt lazily, only when `state_dirty` is set (cheap to
   read, expensive to build, but rebuilds are rare after startup).
3. `object_list::ObjectList` — one per tab; the filtered, scrollable, selectable
   slice of the `View` for that tab, plus dropdown state.

Tabs are `config::TabKind` (Playback / Recording / Output / Input /
Configuration); which exist and their order come from `config.tabs`. Leaf
widgets: `node_widget`, `device_widget`, `dropdown_widget`, `meter` (peak
meters), `help`.

### Events and actions

The `Handle` trait in `src/app.rs` is the central dispatch: every event type
(`Event`, crossterm key/mouse, `PipewireEvent`, `StateEvent`, and `Action`
itself) implements `handle(self, &mut App) -> Result<bool>`, where the bool
means "needs redraw". `Action` (in `src/app.rs`) is the UI action vocabulary; it
`Deserialize`s straight from keybinding config. Rendering populates
`mouse_areas` (`Rect` + accepted `MouseEventKind`s + `Action`s); mouse events
are resolved by hit-testing those areas, so mouse and keyboard share one action
path.

### Peak metering and lazy capture

Peak meters require attaching capture streams to nodes. The `PeakProcessor`
callback (defined in `App::new`) applies VU-meter ballistics. Peaks live in
`Arc<[AtomicF32]>` shared directly between `State` and `View` — lock-free,
`Ordering::Relaxed` (`src/atomic_f32.rs`) — so peak updates don't dirty the
view. With `lazy_capture` on, `App` only captures on-screen nodes, tracking
`visible_objects` / `capturable_objects` / `capturing_objects` and reacting to
`CaptureEligibility` events. The `peaks` config is `off` / `mono` / `auto`.

### Configuration

TOML, merged onto built-in defaults. `Config::try_new(path, &opt)` loads the
file then lets CLI `Opt` flags override individual fields. `ConfigFile` uses
`#[serde(deny_unknown_fields)]` and custom `deserialize_with` "merge" functions
(keybindings, char_sets, themes, filters) so a user file only specifies
overrides. Sub-modules under `src/config/` cover char sets, themes, keybindings,
name templates (PipeWire-property templating with `overrides` matched on object
properties — see `matching.rs`/`property_key.rs`), and filters (`MatchCondition`).

**`wiremix.toml` at the repo root is both the bundled example and the canonical
reference** for every option and default. Keep it in sync when adding or
changing config; the README points users to it rather than duplicating docs.

### The machine-readable CLI (`src/cli/`)

A one-shot noun-verb CLI lives **alongside** the TUI, built on the same
`wirehose` layer. `src/main.rs` dispatches: a subcommand → `cli::run` (one-shot,
returns an exit code); no subcommand → the existing TUI (== `--format explore`).
The TUI path is left untouched — CLI logic does not leak into `src/app.rs`.

A CLI command spawns a `Session`, pumps the event channel applying
`State::update` until `PipewireEvent::Ready` (with a timeout → structured
error), snapshots `State`, then either projects it to DTOs (reads) or sends a
`Command` and awaits the confirming `StateEvent` (writes), prints, and drops the
`Session` for clean shutdown. Modules: `output` (mode-detection cascade),
`envelope` (`{metadata,data}`), `error` (`ExitCode` + `AppError` with runnable
`hint`), `dto` (stable snake_case projections of `state` structs), `oneshot`
(Session lifecycle), `schema` (`schema`/`describe`).

This is the project's reason-for-existing contract — see `AGENTS.md` for the
exact output rules (stdout=data only, JSON envelope, exit codes, ISO 8601 UTC,
agent-env detection). Unlike the TUI (which ignores PipeWire errors in release),
the CLI **surfaces** errors with a non-zero exit and a structured stderr object.

### The desktop GUI (`src/gui/` + `ui/`, `gui` feature)

A third frontend, **off by default**. The default `wiremix` build pulls in
neither Slint nor a graphics stack; the `gui` feature gates an extra binary,
`wiremix-gui` (`[[bin]] required-features = ["gui"]`), and `build.rs` compiles
`ui/main.slint` only when the feature is on (`#[cfg(feature = "gui")]`). It is
built on [Slint](https://slint.dev) with the **software renderer + winit
backend** (no GPU), so current Slint requires **Rust ≥ 1.88** — hence the dev
shell tracks a recent nixpkgs; the core TUI/CLI MSRV (1.74) is unchanged.

Like the CLI, it drives the same `wirehose` `Session` + `view::View` core and
never touches `pipewire`/`libspa` directly. `src/gui/main.rs` spawns a
`Session`, pumps the `mpsc` event channel from a `slint::Timer` on the UI
thread, applies `State::update`, and on structural change rebuilds a transient
`View` and refills the Slint models (`NodeRow`/`DeviceRow`). High-frequency VU
peaks live in a **separate** `PeakRow` model updated in place each tick, so
meter refreshes never reset the volume sliders. Controls flow out through Slint
`Logic` callbacks carrying the **row index** (mapped back to `ObjectId` via the
lists cached at projection time, so no id round-trips through Slint's 32-bit
`int`), each running a `View` command method against the live session. Capture
streams for metering are started/stopped on `CaptureEligibility` events, exactly
as the TUI does. The TUI (`src/app.rs`) and CLI (`src/cli/`) are left untouched.
The Steelbore theme (Void Navy / Molten Amber, §11) is centralised in a Slint
`Theme` global; the `accessibility` Slint feature is enabled.
