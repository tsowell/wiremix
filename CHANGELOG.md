# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-01-27

### Added

- Optional lazy capturing, to only monitor peak levels of on-screen nodes.

### Changed

- Change default target framerate to 60.
- Optimize redrawing.
- Optimize peak level processing and propagation.

## [0.8.0] - 2025-11-12

### Added

- Desktop entry.

### Fixed

- No longer exits on non-fatal PipeWire "Buffer allocation failed" errors.
- Clear terminal after initialization.
- Fix typos in README, wiremix.toml.

## [0.7.0] - 2025-08-14

### Added

- Arbitrary PipeWire object properties can be used in configuration file.

### Fixed

- Open dropdowns no longer persist after their underlying object is removed.

## [0.6.2] - 2025-07-14

### Fixed

- Prepend "v" to cargo version string.

## [0.6.1] - 2025-07-14

### Fixed

- Fall back to cargo version string for --version when publishing crate.

## [0.6.0] - 2025-07-14

### Added

- "git describe" information in --version output.
- Keybinding help menu.
- max_volume_percent option to set the upper range of volume sliders.
- enforce_max_volume option to prevent increasing volume above
  max_volume_percent.

### Changed

- Command-line help text wraps to terminal size.

### Fixed

- Volume slider rendering when volume slider is full.
- Peak capturing after a node's object id is reused.

## [0.5.0] - 2025-06-26

### Changed

- Get control characters from termios for emulating SIGINT/SIGQUIT/EOF.
- Add client:application.name and client:application.process.binary tags.

## [0.4.0] - 2025-05-18

### Changed

- Combine bindings for opening a dropdown and choosing a dropdown item.

### Fixed

- Fix a problem with ensuring that there is always an object selected.

## [0.3.0] - 2025-05-13

### Added

- Nix package to flake.nix.
- Command-line and configuration file option for setting the initial tab.

### Fixed

- Fix a discrepancy between wiremix.toml char set and real defaults.

## [0.2.0] - 2025-05-05

### Added

- This CHANGELOG file.
- Shift+Tab default keybinding.

### Changed

- Enable LTO and set codegen-units to 1.

## [0.1.1] - 2025-04-30

### Fixed

- Fix typos and outdated information in README and wiremix.toml.

## [0.1.0] - 2025-04-24

### Added

- Initial release of wiremix.
