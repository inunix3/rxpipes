# rxpipes
This program is a 2D screensaver which recreates the Pipes screensaver from old MS Windows versions.

![First screenshot of the screensaver](screenshots/screenshot_1.png)
![Second screenshot of the screensaver](screenshots/screenshot_2.png)

## Features
- Multiple sets of pieces (see the [Piece Sets](#piece-sets) section to see them)
- Each pipe has its own color, the available palettes are: none (colorless), base colors and RGB.
- Change the speed of drawing (FPS).
- The minimal and maximal length of pipes can be specified
- The maximal number of drawn characters can be also specified. To ignore this setting you can specify 0 via CLI.
- The probability of turning pipes is changeable, it's given as a percentage in decimal form.
Specifying 0 will make this program to draw straight pipes and 1 will make your screen look like...
umm... pseudo dragon curve or something like that?

## Installation
You'll need the Rust toolchain ([rustup](https://rustup.rs/) or from system package repo)
and make sure that it's up to date.

Once you have the toolchain prepared, type `cargo install rxpipes`. If the process went well, you
can now run the rxpipes simply by typing `rxpipes`. If the shell says that the command does not
exists, then make sure that `$HOME/.cargo/bin` (or whatever the default cargo dir will be) is in the
`PATH` environment variable.

## Running
Just type `rxpipes`.

## Controls
| Key                  | Action |
|----------------------|--------|
| `q` / `Q` / `Escape` | Quit   |
| `Space`              | Pause  |

## Piece Sets

You can select a set by passing `-P <ID>` to rxpipes.

| ID | Description                     | Image                             |
|----|---------------------------------|-----------------------------------|
| 0  | ASCII pipes                     | ![](screenshots/screenshot_0.png) |
| 1  | Thin dots                       | ![](screenshots/screenshot_1.png) |
| 2  | Bold dots                       | ![](screenshots/screenshot_2.png) |
| 3  | Thin pipes                      | ![](screenshots/screenshot_3.png) |
| 4  | Thin pipes with rounded corners | ![](screenshots/screenshot_4.png) |
| 5  | Double pipes                    | ![](screenshots/screenshot_5.png) |
| 6  | Bold pipes                      | ![](screenshots/screenshot_6.png) |
