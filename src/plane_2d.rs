// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

/// 2D point: `(x, y)`.
#[derive(Copy, Clone, Debug, Default)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

impl Point {
    /// Move a point one unit in the specified direction.
    pub fn advance(&mut self, dir: Direction) {
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
    pub fn wrap(&mut self, width: isize, height: isize) {
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

/// Main four (cardinal) directions.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Direction {
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
