// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use clap::{Parser, ValueEnum};
use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode},
    execute,
    style::{self, Color},
    terminal as term,
};
use eyre::{Result, WrapErr};
use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};
use std::{
    io::{self, Stdout},
    panic::{set_hook, take_hook},
    time::Duration,
};

/// Map of different piece sets.
///
/// Index via `[PIPE_SET_IDX][DIRECTION OF THE PREVIOUS PIECE][CURRENT DIRECTION]`
const PIPE_MAP: [[[char; 4]; 4]; 7] = [
    [
        // Up
        ['|', '|', '+', '+'],
        // Down
        ['|', '|', '+', '+'],
        // Right
        ['+', '+', '-', '-'],
        // Left
        ['+', '+', '-', '-'],
    ],
    [
        // Up
        ['·', '·', '·', '·'],
        // Down
        ['·', '·', '·', '·'],
        // Right
        ['·', '·', '·', '·'],
        // Left
        ['·', '·', '·', '·'],
    ],
    [
        // Up
        ['•', '•', '•', '•'],
        // Down
        ['•', '•', '•', '•'],
        // Right
        ['•', '•', '•', '•'],
        // Left
        ['•', '•', '•', '•'],
    ],
    [
        // Up
        ['│', '│', '┌', '┐'],
        // Down
        ['│', '│', '└', '┘'],
        // Right
        ['┘', '┐', '─', '─'],
        // Left
        ['└', '┌', '─', '─'],
    ],
    [
        // Up
        ['│', '│', '╭', '╮'],
        // Down
        ['│', '│', '╰', '╯'],
        // Right
        ['╯', '╮', '─', '─'],
        // Left
        ['╰', '╭', '─', '─'],
    ],
    [
        // Up
        ['║', '║', '╔', '╗'],
        // Down
        ['║', '║', '╚', '╝'],
        // Right
        ['╝', '╗', '═', '═'],
        // Left
        ['╚', '╔', '═', '═'],
    ],
    [
        // Up
        ['┃', '┃', '┏', '┓'],
        // Down
        ['┃', '┃', '┗', '┛'],
        // Right
        ['┛', '┓', '━', '━'],
        // Left
        ['┗', '┏', '━', '━'],
    ],
];

/// Main four (cardinal) directions.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
enum Direction {
    #[default]
    Up,
    Down,
    Right,
    Left,
}

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..=3) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Right,
            _ => Direction::Left,
        }
    }
}

/// 2D point: `(x, y)`.
#[derive(Copy, Clone, Debug, Default)]
struct Point {
    x: i16,
    y: i16,
}

impl Point {
    /// Move a point one unit in the specified direction.
    fn advance(&mut self, dir: Direction) {
        match dir {
            Direction::Up => self.y -= 1,
            Direction::Down => self.y += 1,
            Direction::Right => self.x += 1,
            Direction::Left => self.x -= 1,
        };
    }

    /// Wrap a point within the plane (specified by width and height).
    ///
    /// E.g. for a plane 24 units wide, the x-coord -28 will be wrapped as 20 units, because if we
    /// are at the 24th point, then after going 24 units left, we will be at the 24th point again.
    /// And if we go another 4 units left, we'll end up at the 20th point.
    fn wrap(&mut self, width: i16, height: i16) {
        let wrap_coord = |x: i16, m: i16| -> i16 {
            if x < 0 {
                m - x.abs() % m
            } else if x >= m {
                x % m
            } else {
                x
            }
        };

        self.x = wrap_coord(self.x, width);
        self.y = wrap_coord(self.y, height);
    }
}

/// Represents a piece of pipe.
#[derive(Copy, Clone, Default, Debug)]
struct PipePiece {
    /// Position of the piece.
    pos: Point,
    /// Direction of the preceeding piece.
    prev_dir: Direction,
    /// Direction of the piece.
    dir: Direction,
    /// Color of the piece.
    color: Option<Color>,
}

impl PipePiece {
    /// Create a `PipePiece` with position `(0, 0)`, unspecified directions and without a color.
    fn new() -> Self {
        Default::default()
    }

    /// Create a piece with random direction and color.
    fn gen(palette: ColorPalette) -> Self {
        let mut rng = thread_rng();
        let initial_dir: Direction = rng.gen();

        Self {
            pos: Point { x: 0, y: 0 },
            prev_dir: initial_dir,
            dir: initial_dir,
            color: gen_color(palette),
        }
    }
}

/// Pick random color from the specified palette.
fn gen_color(palette: ColorPalette) -> Option<Color> {
    let mut rng = thread_rng();

    match palette {
        ColorPalette::None => None,
        ColorPalette::BaseColors => {
            Color::parse_ansi(format!("5;{}", rng.gen_range(0..16)).as_str())
        }
        ColorPalette::Rgb => Some(Color::Rgb {
            r: rng.gen(),
            g: rng.gen(),
            b: rng.gen(),
        }),
    }
}

/// Drawing area of the terminal. Wrapper over crossterm.
#[derive(Debug)]
struct Canvas {
    /// Output destination.
    out: Stdout,
    /// Size of the canvas (width, height).
    size: (u16, u16),
}

impl Canvas {
    /// Create a `Canvas` with specified output destination and size.
    fn new(out: Stdout, size: (u16, u16)) -> Self {
        Self { out, size }
    }

    /// Clear the terminal screen, hide the cursor and enable raw mode (in this mode the
    /// terminal passes the input as-is to the program).
    fn init(&mut self) -> Result<()> {
        self.new_sheet()?;
        execute!(self.out, cursor::Hide).wrap_err("failed to hide the cursor")?;
        term::enable_raw_mode().wrap_err("failed to enable raw mode")
    }

    /// Restore previous state of the terminal; clear the terminal screen, restore the cursor
    /// and disable raw mode.
    fn deinit(&mut self) -> Result<()> {
        term::disable_raw_mode().wrap_err("failed to disable raw mode")?;
        execute!(self.out, cursor::Show).wrap_err("failed to show up the cursor")?;
        self.set_fg_color(Color::Reset)?;
        self.move_to(Point { x: 0, y: 0 })?;
        self.remove_sheet()
    }

    /// Make the terminal blank.
    fn clear(&mut self) -> Result<()> {
        execute!(self.out, term::Clear(term::ClearType::All))
            .wrap_err("failed to clean the terminal")
    }

    /// Move the cursor to the 2D point.
    fn move_to(&mut self, p: Point) -> Result<()> {
        execute!(self.out, cursor::MoveTo(p.x as u16, p.y as u16))
            .wrap_err("failed to move the cursor")
    }

    /// Set the foreground color (i.e., color of characters).
    fn set_fg_color(&mut self, c: Color) -> Result<()> {
        execute!(self.out, style::SetForegroundColor(c))
            .wrap_err("failed to set a foreground color")
    }

    /// Print char at current position of the cursor.
    fn put_char(&mut self, ch: char) -> Result<()> {
        print!("{}", ch);
        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, enter the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn new_sheet(&mut self) -> Result<()> {
        execute!(self.out, term::EnterAlternateScreen).wrap_err("failed to enter the alt screen")
    }

    /// If the `alternate-screen` feature is enabled, leave the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn remove_sheet(&mut self) -> Result<()> {
        execute!(self.out, term::LeaveAlternateScreen).wrap_err("failed to leave the alt screen")
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn new_sheet(&mut self) -> Result<()> {
        self.clear()
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn remove_sheet(&mut self) -> Result<()> {
        self.clear()
    }
}

#[derive(Copy, Clone, Eq, Default, PartialEq, Debug, ValueEnum)]
enum ColorPalette {
    None,
    #[default]
    BaseColors,
    Rgb,
}

/// Screensaver settings and CLI parser.
#[derive(Debug, Parser)]
#[command(
    about = "2D version of the ancient pipes screensaver for terminals.",
    author = "inunix3",
    version = "1.0",
    long_about = None,
)]
struct Config {
    /// Frames per second.
    #[arg(short, long, value_parser = 1.., default_value_t = 20)]
    fps: i64,
    /// Maximum drawn pieces of pipes on the screen.
    /// When this maximum is reached, the screen will be cleared.
    /// Set it to 0 to remove the limit.
    #[arg(short, long, default_value_t = 1000, verbatim_doc_comment)]
    max_drawn_pieces: u32,
    /// Maximum length of pipe in pieces.
    /// Must not equal to or be less than --min-pipe-length.
    #[arg(long, default_value_t = 300, verbatim_doc_comment)]
    max_pipe_length: u32,
    /// Minimal length of pipe in pieces.
    /// Must not equal to or be greater than --max-pipe-length.
    #[arg(long, default_value_t = 7, verbatim_doc_comment)]
    min_pipe_length: u32,
    /// Probability of turning a pipe as a percentage in a decimal form.
    #[arg(short = 't', long, default_value_t = 0.2)]
    turning_prob: f64,
    /// Set of colors used for coloring each pipe.
    /// `None` disables this feature. Base colors are 16 colors predefined by the terminal.
    /// The RGB option is for terminals with true color support (all 16 million colors).
    #[arg(short, long, default_value_t, value_enum, verbatim_doc_comment)]
    palette: ColorPalette,
    /// A set of pieces to use.
    /// Available piece sets:
    /// 0 - ASCII pipes:
    ///     |- ++ ++  +- -+ -|-
    /// 1 - thin dots:
    ///     ·· ·· ··  ·· ·· ···
    /// 2 - bold dots:
    ///     •• •• ••  •• •• •••
    /// 3 - thin pipes:
    ///     │─ ┐└ ┘┌  └─ ─┐ ─│─
    /// 4 - thin pipes with rounded corners:
    ///     │─ ╮╰ ╯╭  ╰─ ─╮ ─│─
    /// 5 - double pipes:
    ///     ║═ ╗╚ ╝╔  ╚═ ═╗ ═║═
    /// 6 - bold pipes (default):
    ///     ┃━ ┓┗ ┛┏  ┗━ ━┓ ━┃━
    /// This parameter expects a numeric ID.
    #[arg(short = 'P', long, default_value_t = 6, value_parser = 0..=6, verbatim_doc_comment)]
    piece_set: i64,
}

/// State of the screensaver.
#[derive(Debug)]
struct State {
    /// Current pipe piece to be drawn.
    pipe_piece: PipePiece,
    /// Number of pieces not drawn yet.
    pieces_remaining: u32,
    /// Number of currently drawn pieces.
    drawn_pieces: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pipe_piece: PipePiece::new(),
            pieces_remaining: 0,
            drawn_pieces: 0,
        }
    }
}

impl State {
    /// Create a `State`.
    fn new() -> Self {
        Default::default()
    }
}

/// Represents the screensaver application.
#[derive(Debug)]
struct Screensaver {
    state: State,
    canv: Canvas,
    cfg: Config,
}

impl Screensaver {
    /// Create a `Screensaver`.
    fn new(canv: Canvas, cfg: Config) -> Self {
        Self {
            state: State::new(),
            canv,
            cfg,
        }
    }

    /// Free all resources.
    fn deinit(&mut self) -> Result<()> {
        self.canv.deinit()
    }

    /// Process a new frame.
    fn update(&mut self) -> Result<()> {
        // Aliases with shorter names
        let state = &mut self.state;
        let canv = &mut self.canv;
        let cfg = &self.cfg;
        let piece = &mut state.pipe_piece;

        let mut rng = thread_rng();

        if state.pieces_remaining == 0 {
            state.pieces_remaining = rng.gen_range(cfg.min_pipe_length..cfg.max_pipe_length);

            *piece = PipePiece::gen(cfg.palette);
            piece.pos = Point {
                x: rng.gen_range(0..canv.size.0) as i16,
                y: rng.gen_range(0..canv.size.1) as i16,
            };

            if let Some(color) = piece.color {
                canv.set_fg_color(color)?;
            }
        }

        piece.pos.advance(piece.dir);
        piece.pos.wrap(canv.size.0 as i16, canv.size.1 as i16);
        piece.prev_dir = piece.dir;

        // Try to turn the pipe to other direction
        if rng.gen_bool(cfg.turning_prob) {
            let choice = rng.gen_bool(0.5);

            piece.dir = match piece.dir {
                Direction::Up | Direction::Down => {
                    if choice {
                        Direction::Right
                    } else {
                        Direction::Left
                    }
                }
                Direction::Right | Direction::Left => {
                    if choice {
                        Direction::Up
                    } else {
                        Direction::Down
                    }
                }
            }
        }

        state.pieces_remaining -= 1;

        Ok(())
    }

    /// Display the current state.
    fn draw(&mut self) -> Result<()> {
        // Aliases with shorter names
        let state = &mut self.state;
        let canv = &mut self.canv;
        let cfg = &self.cfg;
        let piece = &mut state.pipe_piece;

        canv.move_to(piece.pos)?;
        canv.put_char(
            PIPE_MAP[cfg.piece_set as usize][piece.prev_dir as usize][piece.dir as usize],
        )?;

        state.drawn_pieces += 1;

        if state.drawn_pieces == cfg.max_drawn_pieces {
            state.drawn_pieces = 0;
            state.pieces_remaining = 0;

            canv.clear()?;
        }

        Ok(())
    }

    /// Run a main loop in the current thread until an external event is received (a key press or
    /// signal) or some internal error is occurred.
    fn run(&mut self) -> Result<()> {
        let delay = Duration::from_millis(1000 / self.cfg.fps as u64);

        let mut quit = false;
        let mut pause = false;

        while !quit {
            // Handle incoming events.
            //
            // The poll function blocks the thread if the argument is nonzero, so we can use it
            // for a frame rate limit. The only downside is that if the incoming events are
            // received (e.g., a key press or window resize), this function immediately returns,
            // so the delay isn't always the same. But since the user isn't expected to make
            // thousands of key presses or crazily drag the corner of the window while using
            // screensaver, we can ignore this.
            if poll(delay).wrap_err("cannot poll events")? {
                match read().wrap_err("cannot read received external event")? {
                    Event::Key(event) => match event.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => quit = true,
                        KeyCode::Char(' ') => pause = !pause,
                        _ => (),
                    },
                    Event::Resize(w, h) => {
                        self.canv.size = (w, h);
                        self.canv.clear()?;
                    }
                    _ => (),
                }
            }

            if !pause {
                self.update()?;
                self.draw()?;
            }
        }

        Ok(())
    }
}

/// Set a panic hook that will restore the terminal state when the program panics.
fn set_panic_hook() {
    let old_hook = take_hook();

    set_hook(Box::new(move |panic_info| {
        let mut c = Canvas::new(io::stdout(), (0, 0));
        let _ = c.deinit();

        old_hook(panic_info);
    }));
}

/// An entry point.
fn main() -> Result<()> {
    let cfg = Config::parse();

    let mut canv = Canvas::new(
        io::stdout(),
        term::size().wrap_err("failed to query the size of the terminal")?,
    );

    set_panic_hook();

    canv.init()
        .wrap_err("failed to prepare terminal for drawing")?;

    let mut app = Screensaver::new(canv, cfg);
    let r = app.run();

    app.deinit()
        .wrap_err("failed to restore the terminal previous state")?;

    r
}
