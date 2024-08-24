// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

use crate::canvas::Canvas;
use eyre::{Result, WrapErr};
use termwiz::{
    surface::{Change, CursorVisibility},
    terminal::{buffered::BufferedTerminal, SystemTerminal, Terminal},
};

/// Represents a terminal screen.
pub struct TerminalScreen {
    /// Associated terminal.
    term: BufferedTerminal<SystemTerminal>,
    /// Size.
    size: (usize, usize),
}

impl TerminalScreen {
    /// Create a new `TerminalScreen` instance.
    pub fn new(mut term: SystemTerminal) -> Result<Self> {
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
    pub fn init(&mut self) -> Result<()> {
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
    pub fn deinit(&mut self) -> Result<()> {
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
    pub fn resize(&mut self, size: (usize, usize)) {
        self.size = size;
        self.term.resize(size.0, size.1);
    }

    /// Copy canvas buffer to the terminal screen buffer.
    pub fn copy_canvas(&mut self, canv: &Canvas) {
        self.term
            .draw_from_screen(canv.surface(), canv.pos.x as usize, canv.pos.y as usize);
    }

    /// Renders all changes since the last render.
    pub fn render(&mut self) -> Result<()> {
        self.term.flush()?;

        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, enter the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    pub fn enter_alternate_screen(&mut self) -> Result<()> {
        self.term
            .terminal()
            .enter_alternate_screen()
            .wrap_err("failed to enter alternate screen")?;

        Ok(())
    }

    /// If the `alternate-screen` feature is enabled, leave the alternate screen. If it's not,
    /// just clear the terminal screen.
    #[cfg(feature = "alternate-screen")]
    pub fn leave_alternate_screen(&mut self) -> Result<()> {
        self.term
            .terminal()
            .exit_alternate_screen()
            .wrap_err("failed to leave alternate screen")?;

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    pub fn enter_alternate_screen(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    #[cfg(not(feature = "alternate-screen"))]
    pub fn leave_alternate_screen(&mut self) -> Result<()> {
        self.clear();

        Ok(())
    }

    /// Retrieve reference the associated terminal.
    pub fn terminal(&mut self) -> &mut BufferedTerminal<SystemTerminal> {
        &mut self.term
    }

    /// Retrieve the size of the screen.
    pub fn size(&self) -> (usize, usize) {
        self.size
    }
}
