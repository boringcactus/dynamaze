#![deny(missing_docs)]
#![windows_subsystem = "windows"]

//! DynaMaze, a multiplayer game about traversing a shifting maze

extern crate bincode;
extern crate bytes;
extern crate clipboard;
extern crate futures;
extern crate glutin_window;
extern crate graphics;
#[macro_use]
extern crate lazy_static;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;
extern crate rodio;
#[macro_use]
extern crate serde_derive;
extern crate tokio;

use glutin_window::GlutinWindow;
use opengl_graphics::{Filter, GlGraphics, GlyphCache, OpenGL, TextureSettings};
use piston::event_loop::*;
use piston::input::*;
use piston::window::WindowSettings;

pub use crate::board::Board;
pub use crate::board_controller::BoardController;
pub use crate::board_view::{BoardView, BoardViewSettings};
pub use crate::item::Item;
pub use crate::menu_controller::GameController;
pub use crate::menu_view::GameView;
pub use crate::player::{Player, PlayerID};
pub use crate::tile::{Direction, Shape, Tile};

mod board;
mod board_controller;
mod board_view;
mod colors;
mod item;
mod menu;
mod menu_controller;
mod menu_view;
mod net;
mod player;
mod sound;
mod tile;

fn main() {
    let opengl = OpenGL::V3_2;
    let window_size = [800, 600];

    // Create a window
    let mut window: GlutinWindow = WindowSettings::new(
        "DynaMaze",
        window_size,
    )
        .opengl(opengl)
        .exit_on_esc(true)
        .samples(4)
        .resizable(true)
        .build()
        .unwrap();

    // Prepare event loop and OpenGL graphics handle
    let mut events = Events::new(EventSettings::new());
    let mut gl = GlGraphics::new(opengl);

    let mut game_controller = GameController::new();
    let mut game_view = GameView::new([window_size[0] as f64, window_size[1] as f64]);

    let texture_settings = TextureSettings::new().filter(Filter::Nearest);
    let ref mut glyphs = GlyphCache::new("assets/FiraSans-Regular.ttf", (), texture_settings)
        .expect("Could not load font");

    while let Some(e) = events.next(&mut window) {
        game_controller.event(&game_view, &e);
        if let Some(args) = e.render_args() {
            let viewport = args.viewport();
            game_view.board_view.settings.width = viewport.draw_size[0] as f64;
            game_view.board_view.settings.height = viewport.draw_size[1] as f64;
            gl.draw(viewport, |c, g| {
                use graphics::clear;
                clear(colors::LIGHT, g);
                game_view.draw(&game_controller, glyphs, &c, g);
            });
        }
    }
}
