// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use clap::{Parser, ValueEnum};
use eyre::{Result, WrapErr};
use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};
use std::{
    panic::{set_hook, take_hook},
    time::Duration,
};
use termwiz::{
    caps::Capabilities,
    cell::AttributeChange,
    color::{ColorAttribute, SrgbaTuple},
    input::{InputEvent, KeyCode, KeyEvent, Modifiers},
    surface::{Change, CursorVisibility, Position},
    terminal::{buffered::BufferedTerminal, SystemTerminal, Terminal},
};
use unicode_segmentation::UnicodeSegmentation;

/// Map of default piece sets.
const DEFAULT_PIECE_SETS: [[char; 6]; 7] = [
    ['|', '-', '+', '+', '+', '+'],
    ['·', '·', '·', '·', '·', '·'],
    ['•', '•', '•', '•', '•', '•'],
    ['│', '─', '┌', '┐', '└', '┘'],
    ['│', '─', '╭', '╮', '╰', '╯'],
    ['║', '═', '╔', '╗', '╚', '╝'],
    ['┃', '━', '┏', '┓', '┗', '┛'], // default
];

/// Map from directions to indices for indexing default piece sets.
///
/// Index via `[DIRECTION OF THE PREVIOUS PIECE][CURRENT DIRECTION]`
const PIECE_SETS_IDX_MAP: [[usize; 4]; 4] = [
    // Up
    [0, 0, 2, 3],
    // Down
    [0, 0, 4, 5],
    // Right
    [5, 3, 1, 1],
    // Left
    [4, 2, 1, 1],
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
    x: isize,
    y: isize,
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
    fn wrap(&mut self, width: isize, height: isize) {
        let wrap_coord = |x: isize, m: isize| -> isize {
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
    color: Option<ColorAttribute>,
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
fn gen_color(palette: ColorPalette) -> Option<ColorAttribute> {
    let mut rng = thread_rng();

    match palette {
        ColorPalette::None => None,
        ColorPalette::BaseColors => Some(ColorAttribute::PaletteIndex(rng.gen_range(0..16))),
        ColorPalette::Rgb => Some(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            1.0,
        ))),
    }
}

/// Drawing area of the terminal.
struct Canvas {
    /// Associated terminal.
    term: BufferedTerminal<SystemTerminal>,
    /// Size of the canvas (width, height).
    size: (usize, usize),
}

impl Canvas {
    /// Create a `Canvas` with specified destination terminal (whole screen area is covered).
    fn new(term: SystemTerminal) -> Result<Canvas> {
        let mut term = BufferedTerminal::new(term)?;
        let size = term
            .terminal()
            .get_screen_size()
            .wrap_err("failed to query the size of the terminal")
            .map(|s| (s.cols, s.rows))?;

        Ok(Self { term, size })
    }

    /// Clear the terminal screen, hide the cursor and enable raw mode (in this mode the
    /// terminal passes the input as-is to the program).
    fn init(&mut self) -> Result<()> {
        // When Terminal is dropped, it automatically exists the alternate screen.
        self.new_sheet()?;
        self.term
            .terminal()
            .set_raw_mode()
            .wrap_err("failed to set raw mode")?;
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Hidden));

        Ok(())
    }

    /// Restore previous state of the terminal; clear the terminal screen, restore the cursor
    /// and disable raw mode.
    fn deinit(&mut self) -> Result<()> {
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        self.set_fg_color(ColorAttribute::Default);
        self.move_to(Point { x: 0, y: 0 });
        self.term
            .terminal()
            .set_cooked_mode()
            .wrap_err("failed to unset raw mode")?;
        self.remove_sheet()?;

        Ok(())
    }

    /// Show all changes since the last render.
    fn render(&mut self) -> Result<()> {
        self.term
            .flush()
            .wrap_err("failed to flush screen buffer")?;

        Ok(())
    }

    /// Make the terminal blank.
    fn clear(&mut self) {
        self.term
            .add_change(Change::ClearScreen(ColorAttribute::Default));
    }

    /// Move the cursor to the 2D point.
    fn move_to(&mut self, p: Point) {
        self.term.add_change(Change::CursorPosition {
            x: Position::Absolute(p.x as usize),
            y: Position::Absolute(p.y as usize),
        });
    }

    /// Set the foreground color (i.e., color of characters).
    fn set_fg_color(&mut self, c: ColorAttribute) {
        self.term
            .add_change(Change::Attribute(AttributeChange::Foreground(c)));
    }

    /// Print string at the current position of the cursor.
    fn put_str(&mut self, s: impl AsRef<str>) {
        self.term.add_change(Change::Text(String::from(s.as_ref())));
    }

    /// If the `alternate-screen` feature is enabled, enter the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn new_sheet(&mut self) -> Result<()> {
        self.term
            .terminal()
            .enter_alternate_screen()
            .wrap_err("failed to enter alternate screen")?;

        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, leave the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn remove_sheet(&mut self) -> Result<()> {
        self.term
            .terminal()
            .exit_alternate_screen()
            .wrap_err("failed to leave alternate screen")?;

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn new_sheet(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn remove_sheet(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    /// Retrieve a mutable reference to the associated terminal.
    fn terminal(&mut self) -> &mut BufferedTerminal<SystemTerminal> {
        &mut self.term
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
    #[arg(short, long, value_parser = 1.., default_value_t = 24)]
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
    /// A default set of pieces to use.
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
    /// A string representing custom piece set (takes precedence over -P/--piece-set).
    /// The string must have length of 6 characters. Write it according to `│─┌┐└┘`.
    /// This string must define all 6 pieces, otherwise rxpipes will crash.
    /// Unicode grapheme clusters are supported and treated as single characters.
    #[arg(name = "custom-piece-set", short = 'c', long, verbatim_doc_comment)]
    custom_piece_set_: Option<String>,

    // TODO: implement validation of length for custom-piece-set.
    #[clap(skip)]
    custom_piece_set: Option<Vec<String>>,
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
    fn update(&mut self) {
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
                x: rng.gen_range(0..canv.size.0) as isize,
                y: rng.gen_range(0..canv.size.1) as isize,
            };
        }

        piece.pos.advance(piece.dir);
        piece.pos.wrap(canv.size.0 as isize, canv.size.1 as isize);
        piece.prev_dir = piece.dir;

        // Try to turn the pipe in other direction
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
    }

    /// Display the current state.
    fn draw(&mut self) -> Result<()> {
        // Aliases with shorter names
        let state = &mut self.state;
        let canv = &mut self.canv;
        let cfg = &self.cfg;
        let piece = &mut state.pipe_piece;

        canv.move_to(piece.pos);

        if let Some(color) = piece.color {
            canv.set_fg_color(color)
        }

        let piece_idx = PIECE_SETS_IDX_MAP[piece.prev_dir as usize][piece.dir as usize];

        if let Some(pieces) = &cfg.custom_piece_set {
            canv.put_str(&pieces[piece_idx]);
        } else {
            canv.put_str(DEFAULT_PIECE_SETS[cfg.piece_set as usize][piece_idx].to_string());
        }

        state.drawn_pieces += 1;

        if state.drawn_pieces == cfg.max_drawn_pieces {
            state.drawn_pieces = 0;
            state.pieces_remaining = 0;

            canv.clear();
        }

        canv.render()?;

        Ok(())
    }

    /// Run the main loop in the current thread until an external event is received (a key press or
    /// signal) or some internal error is occurred.
    fn run(&mut self) -> Result<()> {
        let delay = Duration::from_millis(1000 / self.cfg.fps as u64);

        let mut quit = false;
        let mut pause = false;

        while !quit {
            // Handle incoming events.
            //
            // The poll_input function blocks the thread if the argument is nonzero, so we can use it
            // for a frame rate limit. The only downside is that if the incoming events are
            // received (e.g., a key press or window resize), this function immediately returns,
            // so the delay isn't always the same. But since the user isn't expected to make
            // thousands of key presses or crazily drag the corner of the window while using
            // screensaver, we can ignore this.
            if let Some(event) = self
                .canv
                .terminal()
                .terminal()
                .poll_input(Some(delay))
                .wrap_err("cannot read incoming events")?
            {
                match event {
                    InputEvent::Key(KeyEvent {
                        key,
                        modifiers: Modifiers::NONE,
                    }) => match key {
                        KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => quit = true,
                        KeyCode::Char(' ') => pause = !pause,
                        KeyCode::Char('c') => self.canv.clear(),
                        _ => {}
                    },
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Char('c'),
                        modifiers: Modifiers::CTRL,
                    }) => quit = true,
                    InputEvent::Resized { cols, rows } => {
                        self.canv.size = (cols, rows);
                        self.canv.clear();
                    }
                    _ => {}
                }
            }

            if !pause {
                self.update();
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
        let term = SystemTerminal::new_from_stdio(Capabilities::new_from_env().unwrap()).unwrap();
        let mut c = Canvas::new(term).unwrap();
        let _ = c.deinit();

        old_hook(panic_info);
    }));
}

fn parse_cli() -> Config {
    let mut cfg = Config::parse();

    if let Some(s) = &cfg.custom_piece_set_ {
        cfg.custom_piece_set = Some(
            s.graphemes(true) // true here means iterate over extended grapheme clusters (UAX #29).
                .map(|s| s.to_string())
                .collect(),
        );
    }

    cfg
}

/// An entry point.
fn main() -> Result<()> {
    let cfg = parse_cli();

    let term = SystemTerminal::new_from_stdio(
        Capabilities::new_from_env().wrap_err("cannot read terminal capabilities")?,
    )
    .wrap_err("failed to associate terminal with screen buffer")?;
    let mut canv = Canvas::new(term).wrap_err("failed to create canvas")?;

    set_panic_hook();

    canv.init()
        .wrap_err("failed to prepare terminal for drawing")?;

    eprintln!("{} {}", canv.size.0, canv.size.1);

    let mut app = Screensaver::new(canv, cfg);
    let r = app.run();

    app.deinit()
        .wrap_err("failed to restore the terminal previous state")?;

    r
}
