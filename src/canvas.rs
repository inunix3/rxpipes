// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use crate::plane_2d::Point;
use termwiz::{
    cell::AttributeChange,
    color::{ColorAttribute, SrgbaTuple},
    surface::{Change, Position, Surface},
};

/// Drawing area of the terminal.
pub struct Canvas {
    /// Cell buffer.
    surface: Surface,
    /// Size of the canvas.
    size: (usize, usize),
    /// Position of the canvas.
    pub pos: Point,
}

impl Canvas {
    /// Create a `Canvas` with specified size.
    pub fn new(pos: Point, size: (usize, usize)) -> Self {
        let surface = Surface::new(size.0, size.1);

        Self { surface, size, pos }
    }

    /// Resize canvas to specified size.
    pub fn resize(&mut self, size: (usize, usize)) {
        self.size = size;
        self.surface.resize(size.0, size.1);
    }

    /// Make the canvas blank.
    pub fn clear(&mut self) {
        self.surface
            .add_change(Change::ClearScreen(ColorAttribute::Default));
    }

    /// Move the cursor to the 2D point.
    pub fn move_to(&mut self, p: Point) {
        self.surface.add_change(Change::CursorPosition {
            x: Position::Absolute(p.x as usize),
            y: Position::Absolute(p.y as usize),
        });
    }

    /// Set the foreground color of new cells.
    pub fn set_fg_color(&mut self, c: ColorAttribute) {
        self.surface
            .add_change(Change::Attribute(AttributeChange::Foreground(c)));
    }

    /// Set the background color of new cells.
    pub fn set_bg_color(&mut self, c: ColorAttribute) {
        self.surface
            .add_change(Change::Attribute(AttributeChange::Background(c)));
    }

    /// Print string at the current position of the cursor.
    pub fn put_str(&mut self, s: impl AsRef<str>) {
        self.surface
            .add_change(Change::Text(String::from(s.as_ref())));
    }

    /// Makes all characters darker upto the minimal color.
    pub fn darken(&mut self, factor: f32, min: SrgbaTuple) {
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

    /// Retrieve the size of the area.
    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    /// Retrieve a reference to the buffer.
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
}
