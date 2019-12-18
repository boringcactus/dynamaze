#![deny(missing_docs)]
#![windows_subsystem = "windows"]

//! DynaMaze, a multiplayer game about traversing a shifting maze

#[macro_use]
extern crate lazy_static;
// Draw some multi-colored geometry to the screen
// This is a good place to get a feel for the basic structure of a Quicksilver app
extern crate quicksilver;
extern crate rand;
extern crate serde;
extern crate tokio;

use std::env;

use quicksilver::{
    geom::{Circle, Line, Rectangle, Transform, Triangle, Vector},
    graphics::{Background::Col, Color},
    lifecycle::{run, Settings, State, Window},
    Result,
};

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
mod demo;
//mod gamepad;
mod menu;
mod menu_controller;
mod menu_view;
mod net;
mod options;
mod player;
mod sound;
mod tile;
mod tutorial;

// A unit struct that we're going to use to run the Quicksilver functions
// If we wanted to store persistent state, we would put it in here.
struct DrawGeometry;

impl State for DrawGeometry {
    // Initialize the struct
    fn new() -> Result<DrawGeometry> {
        Ok(DrawGeometry)
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        // Remove any lingering artifacts from the previous frame
        window.clear(Color::WHITE)?;
        // Draw a rectangle with a top-left corner at (100, 100) and a width and height of 32 with
        // a blue background
        window.draw(&Rectangle::new((100, 100), (32, 32)), Col(Color::BLUE));
        // Draw another rectangle, rotated by 45 degrees, with a z-height of 10
        window.draw_ex(&Rectangle::new((400, 300), (32, 32)), Col(Color::BLUE), Transform::rotate(45), 10);
        // Draw a circle with its center at (400, 300) and a radius of 100, with a background of
        // green
        window.draw(&Circle::new((400, 300), 100), Col(Color::GREEN));
        // Draw a line with a thickness of 2 pixels, a red background,
        // and a z-height of 5
        window.draw_ex(
            &Line::new((50, 80), (600, 450)).with_thickness(2.0),
            Col(Color::RED),
            Transform::IDENTITY,
            5,
        );
        // Draw a triangle with a red background, rotated by 45 degrees, and scaled down to half
        // its size
        window.draw_ex(
            &Triangle::new((500, 50), (450, 100), (650, 150)),
            Col(Color::RED),
            Transform::rotate(45) * Transform::scale((0.5, 0.5)),
            0,
        );
        // We completed with no errors
        Ok(())
    }
}

// The main isn't that important in Quicksilver: it just serves as an entrypoint into the event
// loop
fn main() {
    // Run with DrawGeometry as the event handler, with a window title of 'Draw Geometry' and a
    // size of (800, 600)
    run::<GameController>("DynaMaze", Vector::new(800, 600), Settings::default());
}

#[cfg(unix)]
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
    let mut glyphs = GlyphCache::new("static/FiraSans-Regular.ttf", (), texture_settings)
        .expect("Could not load font");

    let mut ui = conrod_core::UiBuilder::new([window_size[0].into(), window_size[1].into()])
        .theme(game_view.theme())
        .build();
    ui.fonts.insert_from_file("static/FiraSans-Regular.ttf").unwrap();

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

/// Checks to see if the game was launched with the given command-line argument
pub fn has_arg(arg: &str) -> bool {
    env::args().any(|x| x == arg)
}
