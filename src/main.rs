// Copyright (c) 2024 inunix3
//
// This file is licensed under the MIT License (see LICENSE.md).

mod canvas;
mod color;
mod config;
mod pipe;
mod plane_2d;
mod screensaver;
mod terminal;

use crate::{config::Config, screensaver::Screensaver, terminal::TerminalScreen};
use clap::Parser;
use eyre::{Result, WrapErr};
use std::panic::{set_hook, take_hook};
use termwiz::{caps::Capabilities, terminal::SystemTerminal};
use unicode_segmentation::UnicodeSegmentation;

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
