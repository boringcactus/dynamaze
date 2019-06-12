#![deny(missing_docs)]
#![windows_subsystem = "windows"]

//! DynaMaze, a multiplayer game about traversing a shifting maze

#[macro_use]
extern crate conrod_core;
extern crate conrod_piston;
extern crate glutin_window;
extern crate graphics;
#[macro_use]
extern crate lazy_static;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;
extern crate serde;
extern crate tokio;

use glutin_window::GlutinWindow;
use opengl_graphics::{Filter, GlGraphics, GlyphCache, OpenGL, Texture, TextureSettings};
use piston::event_loop::*;
use piston::input::*;
use piston::window::{Window, WindowSettings};

pub use crate::board::Board;
pub use crate::board_controller::{BoardController, BoardSettings};
pub use crate::board_view::{BoardView, BoardViewSettings};
pub use crate::menu_controller::GameController;
pub use crate::menu_view::GameView;
pub use crate::player::{Player, PlayerID};
pub use crate::tile::{Direction, Shape, Tile};

mod anim;
mod board;
mod board_controller;
mod board_view;
mod colors;
mod gamepad;
mod menu;
mod menu_controller;
mod menu_view;
mod net;
mod options;
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
        .exit_on_esc(false)
        .samples(4)
        .resizable(true)
        .build()
        .unwrap();

    // Prepare event loop and OpenGL graphics handle
    let mut events = Events::new(EventSettings::new());
    let mut gl = GlGraphics::new(opengl);

    let mut game_controller = GameController::new();
    let mut game_view = GameView::new([window_size[0].into(), window_size[1].into()]);

    let texture_settings = TextureSettings::new().filter(Filter::Nearest);
    let mut glyphs = GlyphCache::new("assets/FiraSans-Regular.ttf", (), texture_settings)
        .expect("Could not load font");

    let mut ui = conrod_core::UiBuilder::new([window_size[0].into(), window_size[1].into()])
        .theme(game_view.theme())
        .build();
    ui.fonts.insert_from_file("assets/FiraSans-Regular.ttf").unwrap();

    let mut text_vertex_data = Vec::new();
    let (mut glyph_cache, mut text_texture_cache) = {
        const SCALE_TOLERANCE: f32 = 0.1;
        const POSITION_TOLERANCE: f32 = 0.1;
        let cache = conrod_core::text::GlyphCache::builder()
            .dimensions(window_size[0], window_size[1])
            .scale_tolerance(SCALE_TOLERANCE)
            .position_tolerance(POSITION_TOLERANCE)
            .build();
        let buffer_len = window_size[0] as usize * window_size[1] as usize;
        let init = vec![128; buffer_len];
        let settings = TextureSettings::new();
        let texture = Texture::from_memory_alpha(&init, window_size[0], window_size[1], &settings).unwrap();
        (cache, texture)
    };

    let image_map = conrod_core::image::Map::new();

    let ids = menu_controller::Ids::new(ui.widget_id_generator());

    let mut gamepad = gamepad::Handler::new();

    while let Some(e) = events.next(&mut window) {
        // conrod
        let size = window.size();
        let (win_w, win_h) = (size.width as conrod_core::Scalar, size.height as conrod_core::Scalar);
        if let Some(e) = conrod_piston::event::convert(e.clone(), win_w, win_h) {
            ui.handle_event(e);
        }

        e.update(|_| {
            let mut ui = ui.set_widgets();
            game_controller.gui(&mut ui, &ids);
        });

        // process this event
        game_controller.event(&game_view, &e);

        // if updating...
        if e.update_args().is_some() {
            // peek for gamepad events (remapped to keyboard events automatically)
            while let Some(e) = gamepad.next_event() {
                game_controller.event(&game_view, &e);
            }
        }

        if let Some(args) = e.render_args() {
            let viewport = args.viewport();
            game_view.board_view.settings.width = viewport.draw_size[0].into();
            game_view.board_view.settings.height = viewport.draw_size[1].into();
            ui.win_w = viewport.draw_size[0].into();
            ui.win_h = viewport.draw_size[1].into();
            gl.draw(viewport, |c, g| {
                use graphics::clear;
                clear(colors::LIGHT.into(), g);

                // conrod
                let primitives = ui.draw();
                let cache_queued_glyphs = |_: &mut GlGraphics,
                                           cache: &mut Texture,
                                           rect: conrod_core::text::rt::Rect<u32>,
                                           data: &[u8]|
                    {
                        let offset = [rect.min.x, rect.min.y];
                        let size = [rect.width(), rect.height()];
                        let format = opengl_graphics::Format::Rgba8;
                        text_vertex_data.clear();
                        text_vertex_data.extend(data.iter().flat_map(|&b| vec![255, 255, 255, b]));
                        opengl_graphics::UpdateTexture::update(cache, &mut (), format, &text_vertex_data[..], offset, size)
                            .expect("failed to update texture")
                    };

                fn texture_from_image<T>(img: &T) -> &T { img }

                conrod_piston::draw::primitives(primitives,
                                                c,
                                                g,
                                                &mut text_texture_cache,
                                                &mut glyph_cache,
                                                &image_map,
                                                cache_queued_glyphs,
                                                texture_from_image);


                game_view.draw(&game_controller, &mut glyphs, &c, g);
            });
        }
    }
}
