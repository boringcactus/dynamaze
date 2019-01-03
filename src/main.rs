#![deny(missing_docs)]

//! DynaMaze, a multiplayer game about traversing a shifting maze

extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate rand;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL, Filter, GlyphCache, TextureSettings };
pub use crate::board::Board;
pub use crate::board_controller::BoardController;
pub use crate::board_view::{BoardView, BoardViewSettings};
pub use crate::tile::{Tile, Direction};

mod board;
mod board_controller;
mod board_view;
mod tile;

fn main() {
    let opengl = OpenGL::V3_2;

    // Create a window
    let mut window: Window = WindowSettings::new(
        "DynaMaze",
        [512, 512]
    )
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // Prepare event loop and OpenGL graphics handle
    let mut events = Events::new(EventSettings::new());
    let mut gl = GlGraphics::new(opengl);

    let board = Board::new();
    let mut board_controller = BoardController::new(board);
    let board_view_settings = BoardViewSettings::new();
    let board_view = BoardView::new(board_view_settings);

    let texture_settings = TextureSettings::new().filter(Filter::Nearest);
    let ref mut glyphs = GlyphCache::new("assets/FiraSans-Regular.ttf", (), texture_settings)
        .expect("Could not load font");

    while let Some(e) = events.next(&mut window) {
        board_controller.event(board_view.settings.position, board_view.settings.size, &e);
        if let Some(args) = e.render_args() {
            gl.draw(args.viewport(), |c, g| {
                use graphics::{clear};
                clear([1.0; 4], g);
                board_view.draw(&board_controller, glyphs, &c, g);
            });
        }
    }
}
