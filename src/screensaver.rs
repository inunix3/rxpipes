// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use crate::{
    canvas::Canvas,
    color::GradientDir,
    config::Config,
    pipe::PipePiece,
    plane_2d::{Direction, Point},
    terminal::TerminalScreen,
};
use eyre::{Result, WrapErr};
use hex_color::HexColor;
use rand::{thread_rng, Rng};
use std::time::Duration;
use termwiz::{
    color::{ColorAttribute, SrgbaTuple},
    input::{InputEvent, KeyCode, KeyEvent, Modifiers},
    terminal::Terminal,
};

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
pub struct Screensaver {
    state: State,
    term_scr: TerminalScreen,
    canv: Canvas,
    darken_min: SrgbaTuple,
    bg_color: Option<SrgbaTuple>,
    stats_canv: Canvas,
    cfg: Config,
}

impl Screensaver {
    /// Create a `Screensaver`.
    pub fn new(term_scr: TerminalScreen, cfg: Config) -> Result<Self> {
        let scr_size = term_scr.size();

        let mut s = Ok(Self {
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
            bg_color: {
                if let Some(c) = &cfg.bg_color {
                    let hc = HexColor::parse_rgb(&c)?;

                    Some(SrgbaTuple(
                        hc.r as f32 / 255.0,
                        hc.g as f32 / 255.0,
                        hc.b as f32 / 255.0,
                        hc.a as f32 / 255.0,
                    ))
                } else {
                    None
                }
            },
            stats_canv: Canvas::new(
                Point {
                    x: 0,
                    y: scr_size.1 as isize - 1,
                },
                (scr_size.0, 3),
            ),
            cfg,
        });

        if let Ok(ref mut s) = s {
            s.draw_bg();
        }
        s
    }

    /// Free all resources.
    pub fn deinit(&mut self) -> Result<()> {
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
                x: rng.gen_range(0..canv.size().0) as isize,
                y: rng.gen_range(0..canv.size().1) as isize,
            };

            if state.pieces_total > 0 {
                state.pipes_total += 1;
            }

            state.currently_drawn_pieces = 0;
        }

        piece.pos.advance(piece.dir);
        piece
            .pos
            .wrap(canv.size().0 as isize, canv.size().1 as isize);
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
            let color = if cfg.gradient {
                let step = match piece.gradient {
                    GradientDir::Up => cfg.gradient_step,
                    GradientDir::Down => -cfg.gradient_step,
                };

                let srgba = if let ColorAttribute::TrueColorWithDefaultFallback(srgba) = color {
                    let r = (srgba.0 + step).clamp(0.0, 1.0);
                    let g = (srgba.1 + step).clamp(0.0, 1.0);
                    let b = (srgba.2 + step).clamp(0.0, 1.0);

                    SrgbaTuple(r, g, b, 1.0)
                } else {
                    unreachable!()
                };

                ColorAttribute::TrueColorWithDefaultFallback(srgba)
            } else {
                color
            };

            piece.color = Some(color);
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
        } else if cfg.depth_mode && state.layer_pieces_total >= cfg.layer_max_drawn_pieces {
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
        self.state.pipes_total = 0;

        self.draw_bg();
    }

    fn draw_bg(&mut self) {
        if let Some(c) = self.bg_color {
            self.canv
                .fill(ColorAttribute::TrueColorWithDefaultFallback(c));
        } else {
            self.canv.fill(ColorAttribute::Default);
        }
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
    pub fn run(&mut self) -> Result<()> {
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
                    KeyCode::Char('l') => self.redraw()?,
                    KeyCode::Char('s') => self.cfg.show_stats = !self.cfg.show_stats,
                    _ => {}
                },
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('c'),
                    modifiers: Modifiers::CTRL,
                }) => self.state.quit = true,
                InputEvent::Resized { cols, rows } => {
                    self.canv.resize((cols, rows));
                    self.draw_bg();

                    // self.stats_canv.resize((cols, self.stats_canv.size().1));
                    self.stats_canv.pos.y = rows as isize - 1;
                    self.stats_canv.resize((cols, self.stats_canv.size().1));
                    self.term_scr.resize((cols, rows));

                    self.redraw()?
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        self.term_scr.clear();
        self.render()?;

        Ok(())
    }

    /// Draw a stats widget which shows pipe/piece/layers counters and the current pipe color.
    fn draw_stats(&mut self) {
        // Stats string will have a black background
        self.stats_canv.fill(ColorAttribute::PaletteIndex(0));
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
    }
}
