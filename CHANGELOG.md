# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Add "git describe" information to --version output.

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
