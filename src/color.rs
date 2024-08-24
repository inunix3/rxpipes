// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use clap::ValueEnum;

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub enum GradientDir {
    #[default]
    Up,
    Down,
}

impl Distribution<GradientDir> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GradientDir {
        match rng.gen_range(0..=3) {
            0 => GradientDir::Up,
            _ => GradientDir::Down,
        }
    }
}

#[derive(Copy, Clone, Eq, Default, PartialEq, Debug, ValueEnum)]
pub enum ColorPalette {
    None,
    #[default]
    BaseColors,
    Rgb,
}
