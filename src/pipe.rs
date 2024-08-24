// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use crate::{
    color::{ColorPalette, GradientDir},
    plane_2d::{Direction, Point},
};
use rand::{thread_rng, Rng};
use termwiz::color::{ColorAttribute, SrgbaTuple};

/// Represents a piece of pipe.
#[derive(Copy, Clone, Default, Debug)]
pub struct PipePiece {
    /// Position of the piece.
    pub pos: Point,
    /// Direction of the preceeding piece.
    pub prev_dir: Direction,
    /// Direction of the piece.
    pub dir: Direction,
    /// Color of the piece.
    pub color: Option<ColorAttribute>,
    /// Gradient direction.
    pub gradient: GradientDir,
}

impl PipePiece {
    /// Create a `PipePiece` with position `(0, 0)`, unspecified directions and without a color.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a piece with random direction and color.
    pub fn gen(palette: ColorPalette) -> Self {
        let mut rng = thread_rng();
        let initial_dir: Direction = rng.gen();

        Self {
            pos: Point { x: 0, y: 0 },
            prev_dir: initial_dir,
            dir: initial_dir,
            color: gen_color(palette),
            gradient: rng.gen(),
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
