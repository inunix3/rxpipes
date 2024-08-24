// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use clap::{Parser, ValueEnum};
use eyre::{Result, WrapErr};
use hex_color::HexColor;
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
    surface::{Change, CursorVisibility, Position, Surface},
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

/// Represents a terminal screen.
struct TerminalScreen {
    /// Associated terminal.
    term: BufferedTerminal<SystemTerminal>,
    /// Size.
    size: (usize, usize),
}

impl TerminalScreen {
    /// Create a new `TerminalScreen` instance.
    fn new(mut term: SystemTerminal) -> Result<Self> {
        let size = term
            .get_screen_size()
            .wrap_err("failed to query the size of the terminal")
            .map(|s| (s.cols, s.rows))?;

        Ok(Self {
            term: BufferedTerminal::new(term)?,
            size,
        })
    }

    /// Initialize the terminal screen - enables alternate screen / clear screen, sets raw mode and hides cursor.
    fn init(&mut self) -> Result<()> {
        self.enter_alternate_screen()?;
        self.term
            .terminal()
            .set_raw_mode()
            .wrap_err("failed to set raw mode")?;
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Hidden));

        Ok(())
    }

    /// Restore previous state of the terminal; exit alternate screen / clear the terminal screen,
    /// restore the cursor and disable raw mode.
    fn deinit(&mut self) -> Result<()> {
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        self.term
            .terminal()
            .set_cooked_mode()
            .wrap_err("failed to unset raw mode")?;
        self.leave_alternate_screen()?;

        Ok(())
    }

    /// Resize terminal screen buffer to specified size.
    fn resize(&mut self, size: (usize, usize)) {
        self.size = size;
        self.term.resize(size.0, size.1);
    }

    /// Copy canvas buffer to the terminal screen buffer.
    fn copy_canvas(&mut self, canv: &Canvas) {
        self.term
            .draw_from_screen(&canv.surface, canv.pos.x as usize, canv.pos.y as usize);
    }

    /// Renders all changes since the last render.
    fn render(&mut self) -> Result<()> {
        self.term.flush()?;

        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, enter the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn enter_alternate_screen(&mut self) -> Result<()> {
        self.term
            .terminal()
            .enter_alternate_screen()
            .wrap_err("failed to enter alternate screen")?;

        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, leave the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    fn leave_alternate_screen(&mut self) -> Result<()> {
        self.term
            .terminal()
            .exit_alternate_screen()
            .wrap_err("failed to leave alternate screen")?;

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn enter_alternate_screen(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    fn leave_alternate_screen(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    /// Retrieve reference the associated terminal.
    fn terminal(&mut self) -> &mut BufferedTerminal<SystemTerminal> {
        &mut self.term
    }
}

/// Drawing area of the terminal.
struct Canvas {
    /// Cell buffer.
    surface: Surface,
    /// Size of the canvas.
    size: (usize, usize),
    /// Position of the canvas.
    pos: Point,
}

impl Canvas {
    /// Create a `Canvas` with specified size.
    fn new(pos: Point, size: (usize, usize)) -> Self {
        let surface = Surface::new(size.0, size.1);

        Self { surface, size, pos }
    }

    /// Resize canvas to specified size.
    fn resize(&mut self, size: (usize, usize)) {
        self.size = size;
        self.surface.resize(size.0, size.1);
    }

    /// Make the canvas blank.
    fn clear(&mut self) {
        self.surface
            .add_change(Change::ClearScreen(ColorAttribute::Default));
    }

    /// Move the cursor to the 2D point.
    fn move_to(&mut self, p: Point) {
        self.surface.add_change(Change::CursorPosition {
            x: Position::Absolute(p.x as usize),
            y: Position::Absolute(p.y as usize),
        });
    }

    /// Set the foreground color of new cells.
    fn set_fg_color(&mut self, c: ColorAttribute) {
        self.surface
            .add_change(Change::Attribute(AttributeChange::Foreground(c)));
    }

    /// Set the background color of new cells.
    fn set_bg_color(&mut self, c: ColorAttribute) {
        self.surface
            .add_change(Change::Attribute(AttributeChange::Background(c)));
    }

    /// Print string at the current position of the cursor.
    fn put_str(&mut self, s: impl AsRef<str>) {
        self.surface
            .add_change(Change::Text(String::from(s.as_ref())));
    }

    /// Makes all characters darker upto the minimal color.
    fn darken(&mut self, factor: f32, min: SrgbaTuple) {
        let mut changes: Vec<Change> = vec![];

        for (i, l) in self.surface.screen_cells().iter().enumerate() {
            for (j, cell) in l.iter().enumerate() {
                if cell.str().trim_ascii().is_empty() {
                    continue;
                }

                let attrs = cell.attrs();
                let mut fg = attrs.foreground();

                fg = match fg {
                    ColorAttribute::TrueColorWithDefaultFallback(mut srgba) => {
                        srgba.0 *= factor;
                        srgba.0 = srgba.0.clamp(min.0, 1.0);
                        srgba.1 *= factor;
                        srgba.1 = srgba.1.clamp(min.1, 1.0);
                        srgba.2 *= factor;
                        srgba.2 = srgba.2.clamp(min.2, 1.0);

                        ColorAttribute::TrueColorWithDefaultFallback(srgba)
                    }
                    _ => fg,
                };

                // In order to apply the foreground change, we need so print something.
                let text = cell.str().to_string();

                changes.push(Change::CursorPosition {
                    x: Position::Absolute(j),
                    y: Position::Absolute(i),
                });

                changes.push(Change::Attribute(AttributeChange::Foreground(fg)));
                changes.push(Change::Text(text));
            }
        }

        self.surface.add_changes(changes);
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
    #[arg(short, long, default_value_t = 10000, verbatim_doc_comment)]
    max_drawn_pieces: u64,
    /// Maximum length of pipe in pieces.
    /// Must not equal to or be less than --min-pipe-length.
    #[arg(long, default_value_t = 300, verbatim_doc_comment)]
    max_pipe_length: u64,
    /// Minimal length of pipe in pieces.
    /// Must not equal to or be greater than --max-pipe-length.
    #[arg(long, default_value_t = 7, verbatim_doc_comment)]
    min_pipe_length: u64,
    /// Probability of turning a pipe as a percentage in a decimal form.
    #[arg(short = 't', long, default_value_t = 0.2)]
    turning_prob: f64,
    /// Set of colors used for coloring each pipe.
    /// `None` disables this feature. Base colors are 16 colors predefined by the terminal.
    /// The RGB option is for terminals with true color support (all 16 million colors).
    #[arg(short, long, default_value_t, value_enum, verbatim_doc_comment)]
    palette: ColorPalette,
    /// In this mode multiple layers of pipes are drawn. If the number of currently drawn pieces in
    /// layer is >= layer_max_drawn_pieces, all pipe pieces are made darker and a new layer is created
    /// on top of them. See also darken_factor and darken_min.
    #[arg(short, long)]
    depth_mode: bool,
    /// Depth-mode: maximum drawn pipe pieces in the current layer.
    #[arg(long, default_value_t = 1000)]
    layer_max_drawn_pieces: u64,
    /// Depth-mode: how much to darken pipe pieces in previous layers?
    #[arg(short = 'F', long, default_value_t = 0.8)]
    darken_factor: f32,
    /// Depth-mode: the color to gradually darken to.
    #[arg(short = 't', long, default_value = "#000000")]
    darken_min: String,
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
    /// Show statistics in the bottom of screen (how many pieces drawn, pipes drawn, etc.)
    #[arg(short = 's', long)]
    show_stats: bool,

    // TODO: implement validation of length for custom-piece-set.
    #[clap(skip)]
    custom_piece_set: Option<Vec<String>>,
}

/// State of the screensaver.
#[derive(Debug)]
struct State {
    /// Current pipe piece to be drawn.
    pipe_piece: PipePiece,
    /// Total of all drawn pieces.
    pieces_total: u64,
    /// Total of all drawn pieces in the current layer.
    layer_pieces_total: u64,
    /// Number of currently drawn pieces.
    currently_drawn_pieces: u64,
    /// Number of pieces not drawn yet.
    pieces_remaining: u64,
    /// Total of all drawn pipes.
    pipes_total: u64,
    /// Total of all drawn layers since last screen clear.
    layers_drawn: u64,
    /// Indicates when to end the main loop.
    quit: bool,
    /// Indicates when to stop updating the state.
    pause: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pipe_piece: PipePiece::new(),
            pieces_total: 0,
            layer_pieces_total: 0,
            currently_drawn_pieces: 0,
            pieces_remaining: 0,
            pipes_total: 0,
            layers_drawn: 0,
            quit: false,
            pause: false,
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
    term_scr: TerminalScreen,
    canv: Canvas,
    darken_min: SrgbaTuple,
    stats_canv: Canvas,
    cfg: Config,
}

impl Screensaver {
    /// Create a `Screensaver`.
    fn new(term_scr: TerminalScreen, cfg: Config) -> Result<Self> {
        let scr_size = term_scr.size;

        Ok(Self {
            state: State::new(),
            term_scr,
            canv: Canvas::new(Point { x: 0, y: 0 }, scr_size),
            darken_min: {
                let hc = HexColor::parse_rgb(&cfg.darken_min)?;

                SrgbaTuple(
                    hc.r as f32 / 255.0,
                    hc.g as f32 / 255.0,
                    hc.b as f32 / 255.0,
                    1.0,
                )
            },
            stats_canv: Canvas::new(
                Point {
                    x: 0,
                    y: scr_size.1 as isize - 1,
                },
                (scr_size.0, 3),
            ),
            cfg,
        })
    }

    /// Free all resources.
    fn deinit(&mut self) -> Result<()> {
        self.term_scr.deinit()
    }

    /// Generate the next pipe pieces.
    fn gen_next_piece(&mut self) {
        // Aliases with shorter names
        let state = &mut self.state;
        let canv = &mut self.canv;
        let cfg = &self.cfg;
        let piece = &mut state.pipe_piece;

        let mut rng = thread_rng();

        if state.pieces_remaining == 0 {
            state.pieces_remaining = rng.gen_range(cfg.min_pipe_length..=cfg.max_pipe_length);

            *piece = PipePiece::gen(cfg.palette);
            piece.pos = Point {
                x: rng.gen_range(0..canv.size.0) as isize,
                y: rng.gen_range(0..canv.size.1) as isize,
            };

            if state.pieces_total > 0 {
                state.pipes_total += 1;
            }

            state.currently_drawn_pieces = 0;
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
    }

    /// Display the current state.
    fn draw_pipe_piece(&mut self) {
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

        state.pieces_total += 1;
        state.layer_pieces_total += 1;
        state.currently_drawn_pieces += 1;
        state.pieces_remaining -= 1;

        if state.pieces_total >= cfg.max_drawn_pieces {
            self.clear();
        } else if state.layer_pieces_total >= cfg.layer_max_drawn_pieces {
            self.darken_previous_layers();
        }
    }

    /// Clear the screen and reset all pipe/piece/layer counters.
    fn clear(&mut self) {
        self.state.currently_drawn_pieces = 0;
        self.state.pieces_remaining = 0;
        self.state.layer_pieces_total = 0;
        self.state.pieces_total = 0;
        self.state.layers_drawn = 0;

        self.canv.clear();
    }

    /// Make all pipe pieces in previous layers darker.
    fn darken_previous_layers(&mut self) {
        self.state.currently_drawn_pieces = 0;
        self.state.pieces_remaining = 0;
        self.state.layer_pieces_total = 0;
        self.state.layers_drawn += 1;

        self.canv.darken(self.cfg.darken_factor, self.darken_min);
    }

    /// Render pipes and maybe stats.
    fn render(&mut self) -> Result<()> {
        self.term_scr.copy_canvas(&self.canv);

        if self.cfg.show_stats {
            self.term_scr.copy_canvas(&self.stats_canv);
        }

        self.term_scr.render()?;

        Ok(())
    }

    /// Run the main loop in the current thread until an external event is received (a key press or
    /// signal) or some internal error is occurred.
    fn run(&mut self) -> Result<()> {
        let delay = Duration::from_millis(1000 / self.cfg.fps as u64);

        while !self.state.quit {
            self.handle_events(delay)?;

            if !self.state.pause {
                self.gen_next_piece();
                self.draw_pipe_piece();

                if self.cfg.show_stats {
                    self.draw_stats();
                }

                self.render()?;
            }
        }

        Ok(())
    }

    /// Handle input and incoming events.
    fn handle_events(&mut self, delay: Duration) -> Result<()> {
        // The poll_input function blocks the thread if the argument is nonzero, so we can use it
        // for a frame rate limit. The only downside is that if the incoming events are
        // received (e.g., a key press or window resize), this function immediately returns,
        // so the delay isn't always the same. But since the user isn't expected to make
        // thousands of key presses or crazily drag the corner of the window while using
        // screensaver, we can ignore this.
        if let Some(event) = self
            .term_scr
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
                    KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.state.quit = true
                    }
                    KeyCode::Char(' ') => self.state.pause = !self.state.pause,
                    KeyCode::Char('c') => self.clear(),
                    KeyCode::Char('s') => self.cfg.show_stats = !self.cfg.show_stats,
                    _ => {}
                },
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('c'),
                    modifiers: Modifiers::CTRL,
                }) => self.state.quit = true,
                InputEvent::Resized { cols, rows } => {
                    self.canv.resize((cols, rows));

                    self.stats_canv.resize((cols, self.stats_canv.size.1));
                    self.term_scr.resize((cols, rows));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Draw a stats widget which shows pipe/piece/layers counters and the current pipe color.
    fn draw_stats(&mut self) {
        self.stats_canv.clear();

        // Stats string will have a black background
        self.stats_canv
            .set_bg_color(ColorAttribute::PaletteIndex(0));
        // Stats string will have a gray foreground
        self.stats_canv
            .set_fg_color(ColorAttribute::PaletteIndex(7));

        let pipe_len = self.state.currently_drawn_pieces + self.state.pieces_remaining;

        let color = self
            .state
            .pipe_piece
            .color
            .map_or("DEFAULT".to_string(), |c| match c {
                ColorAttribute::Default => "DEFAULT".to_string(),
                ColorAttribute::PaletteIndex(i) => match i {
                    0 => "BLACK",
                    1 => "RED",
                    2 => "GREEN",
                    3 => "YELLOW",
                    4 => "BLUE",
                    5 => "MAGENTA",
                    6 => "CYAN",
                    7 => "WHITE",
                    8 => "BRIGHT BLACK",
                    9 => "BRIGHT RED",
                    10 => "BRIGHT GREEN",
                    11 => "BRIGHT YELLOW",
                    12 => "BRIGHT BLUE",
                    13 => "BRIGHT MAGENTA",
                    14 => "BRIGHT CYAN",
                    15 => "BRIGHT GRAY",
                    _ => unreachable!(),
                }
                .to_string(),
                ColorAttribute::TrueColorWithPaletteFallback(c, _)
                | ColorAttribute::TrueColorWithDefaultFallback(c) => c.to_rgb_string(),
            });

        let s = format!(
            "pcs. drawn: {}, lpcs. drawn: {}, c. pcs. drawn: {}, pps. drawn: {}, pcs. rem: {}, l. drawn: {}, pps. len: {}, pipe color: {}",
            self.state.pieces_total,
            self.state.layer_pieces_total,
            self.state.currently_drawn_pieces,
            self.state.pipes_total,
            self.state.pieces_remaining,
            self.state.layers_drawn,
            pipe_len,
            color
        );

        self.stats_canv.put_str(s);
        self.stats_canv.set_bg_color(ColorAttribute::Default);
    }
}

/// Set a panic hook that will restore the terminal state when the program panics.
fn set_panic_hook() {
    let old_hook = take_hook();

    set_hook(Box::new(move |panic_info| {
        let term = SystemTerminal::new_from_stdio(Capabilities::new_from_env().unwrap()).unwrap();
        let mut term_scr = TerminalScreen::new(term).unwrap();
        let _ = term_scr.deinit();

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
    let mut term_scr = TerminalScreen::new(term).wrap_err("cannot set up terminal screen")?;

    set_panic_hook();

    term_scr
        .init()
        .wrap_err("failed to prepare terminal for drawing")?;

    let mut app = Screensaver::new(term_scr, cfg)?;
    let r = app.run();

    app.deinit()
        .wrap_err("failed to restore the terminal previous state")?;

    r
}
