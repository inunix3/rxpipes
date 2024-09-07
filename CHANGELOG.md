# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2024-09-07

### Added

- Keybind `,`: change speed by -1.
- Keybind `.`: change speed by +1.
- Keybind `<`: change speed by -10.
- Keybind `>`: change speed by +10.
- Option `-b`/`--bg-color`: background color (optional, by default transparent).

### Fixed

- Lighten rather than darken if pipe piece color is < than the `--darken-min` value.
- Fix invalid handling of `--max-drawn-pieces = 0`.

## [1.2.0] - 2024-08-24

### Added

- Depth mode (RGB palette only).
- Gradient mode (RGB palette only).
- Stats widget.
- Keybind `Ctrl-C`: exit.
- Keybind `l` (lowercase L): redraw.
- Keybind `s`: toggle the stats widget.
- Option `g`/`--gradient`: enable gradient mode.
- Option `--gradient-step`: step between gradient transitions.
- Option `-d`/`--depth-mode`: enable depth mode.
- Option `--layer-max-drawn-pieces`: maximal number of pieces in the current layer (depth mode).
- Option `-F`/`--darken-factor`: how much to darken pipe pieces in previous layers? (depth mode).
- Option `-M`/`--darken-min`: the color to gradually darken to (depth mode).
- Option `-s`/`--show-stats`: toggle the stats widget.

### Changed

- Changed default FPS from 20 to 24 frames.

## [1.1.0] - 2024-05-09

### Added

- Support for defining custom piece sets.
- Keybind `c`: manual screen cleaning.

### Changed

- The default piece set are now bold pipes (ID: 6).
- Slightly edited the help message.

## [1.0.0] - 2024-05-03

### Added

- 6 available piece sets.
- Each pipe has its own color; available palettes are: none (colorless), base colors and RGB.
- Changeable FPS.
- The minimal and maximal length of pipes can be changed.
- The maximal number of drawn characters can be changed. To ignore this setting specify 0 via CLI.
- The probability of turning pipes is changeable, it's given as a percentage in decimal form.
- Screensaver can be paused by pressing spacebar, close with q, Q or escape key.
