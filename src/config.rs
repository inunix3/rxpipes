// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use crate::color::ColorPalette;
use clap::Parser;

/// Screensaver settings and CLI parser.
#[derive(Debug, Parser)]
#[command(
    about = "2D version of the ancient pipes screensaver for terminals.",
    author = "inunix3",
    version = "1.2.0",
    long_about = None,
)]
pub struct Config {
    /// Frames per second.
    #[arg(short, long, value_parser = 1.., default_value_t = 24)]
    pub fps: i64,
    /// Maximum drawn pieces of pipes on the screen.
    /// When this maximum is reached, the screen will be cleared.
    /// Set it to 0 to remove the limit.
    #[arg(short, long, default_value_t = 10000, verbatim_doc_comment)]
    pub max_drawn_pieces: u64,
    /// Maximum length of pipe in pieces.
    /// Must not equal to or be less than --min-pipe-length.
    #[arg(long, default_value_t = 300, verbatim_doc_comment)]
    pub max_pipe_length: u64,
    /// Minimal length of pipe in pieces.
    /// Must not equal to or be greater than --max-pipe-length.
    #[arg(long, default_value_t = 7, verbatim_doc_comment)]
    pub min_pipe_length: u64,
    /// Probability of turning a pipe as a percentage in a decimal form.
    #[arg(short = 't', long, default_value_t = 0.2)]
    pub turning_prob: f64,
    /// Set of colors used for coloring each pipe.
    /// `None` disables this feature. Base colors are 16 colors predefined by the terminal.
    /// The RGB option is for terminals with true color support (all 16 million colors).
    #[arg(short, long, default_value_t, value_enum, verbatim_doc_comment)]
    pub palette: ColorPalette,
    /// Enable gradient. Use only with RGB palette.
    #[arg(short, long)]
    pub gradient: bool,
    /// Gradient: the step to lighten/darken the color.
    #[arg(long, default_value_t = 0.005)]
    pub gradient_step: f32,
    /// In this mode multiple layers of pipes are drawn. If the number of currently drawn pieces in
    /// layer is >= layer_max_drawn_pieces, all pipe pieces are made darker and a new layer is created
    /// on top of them. See also darken_factor and darken_min. RGB palette only!
    #[arg(short, long, verbatim_doc_comment)]
    pub depth_mode: bool,
    /// Depth-mode: maximum drawn pipe pieces in the current layer.
    #[arg(long, default_value_t = 1000)]
    pub layer_max_drawn_pieces: u64,
    /// Depth-mode: how much to darken pipe pieces in previous layers?
    #[arg(short = 'F', long, default_value_t = 0.8)]
    pub darken_factor: f32,
    /// Depth-mode: the color to gradually darken to.
    #[arg(short = 'M', long, default_value = "#000000")]
    pub darken_min: String,
    /// Color of the background.
    #[arg(short = 'b', long)]
    pub bg_color: Option<String>,
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
    pub piece_set: i64,
    /// A string representing custom piece set (takes precedence over -P/--piece-set).
    /// The string must have length of 6 characters. Write it according to `│─┌┐└┘`.
    /// This string must define all 6 pieces, otherwise rxpipes will crash.
    /// Unicode grapheme clusters are supported and treated as single characters.
    #[arg(name = "custom-piece-set", short = 'c', long, verbatim_doc_comment)]
    pub custom_piece_set_: Option<String>,
    /// Show statistics in the bottom of screen (how many pieces drawn, pipes drawn, etc.)
    #[arg(short = 's', long)]
    pub show_stats: bool,

    // TODO: implement validation of length for custom-piece-set.
    #[clap(skip)]
    pub custom_piece_set: Option<Vec<String>>,
}
