#![deny(missing_docs)]
#![windows_subsystem = "windows"]

//! DynaMaze, a multiplayer game about traversing a shifting maze

#[macro_use]
extern crate lazy_static;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gloo::events::{EventListener, EventListenerOptions};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

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
mod menu;
mod menu_controller;
mod menu_view;
mod net;
mod options;
mod player;
mod sound;
mod tile;
mod tutorial;

fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&JsValue::from_str("started main"));
    let window = web_sys::window().expect("no window");
    let main = {
        let document = window.document().expect_throw("no document");
        document.query_selector("main").expect_throw("failed to find main").expect_throw("failed to find main")
    };

    web_sys::console::log_1(&JsValue::from_str("started controller new"));
    let game_controller = GameController::new();
    web_sys::console::log_1(&JsValue::from_str("finished controller new"));
    let game_controller = Arc::new(Mutex::new(game_controller));

    {
        let game_controller = game_controller.clone();
        let main2 = main.clone();
        let options = EventListenerOptions::enable_prevent_default();
        let click_listener = EventListener::new_with_options(&main, "click", options, move |event| {
            let event = event.dyn_ref::<web_sys::MouseEvent>().expect_throw("bad click event");
            game_controller.lock().unwrap().on_click(event, &main2);
        });
        click_listener.forget();
    }

    {
        let game_controller = game_controller.clone();
        let main2 = main.clone();
        let options = EventListenerOptions::enable_prevent_default();
        let contextmenu_listener = EventListener::new_with_options(&main, "contextmenu", options, move |event| {
            let event = event.dyn_ref::<web_sys::MouseEvent>().expect_throw("bad contextmenu event");
            game_controller.lock().unwrap().on_click(event, &main2);
        });
        contextmenu_listener.forget();
    }

    {
        let game_controller = game_controller.clone();
        let main2 = main.clone();
        let mousemove_listener = EventListener::new(&main, "mousemove", move |event| {
            let event = event.dyn_ref::<web_sys::MouseEvent>().expect_throw("bad mousemove event");
            game_controller.lock().unwrap().on_mousemove(event, &main2);
        });
        mousemove_listener.forget();
    }

    {
        let game_controller = game_controller.clone();
        let main2 = main.clone();
        let keydown_listener = EventListener::new(&main, "keydown", move |event| {
            let event = event.dyn_ref::<web_sys::KeyboardEvent>().expect_throw("bad keydown event");
            game_controller.lock().unwrap().on_keydown(event, &main2);
        });
        keydown_listener.forget();
    }

    // this is *weird* but comes from https://rustwasm.github.io/wasm-bindgen/examples/request-animation-frame.html
    let inner_handle: Rc<RefCell<Option<Closure<_>>>> = Rc::new(RefCell::new(None));
    let outer_handle = inner_handle.clone();

    {
        let window = window.clone();
        let mut last_frame = now();
        *outer_handle.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            last_frame = {
                let this_frame = now();
                let dt = this_frame - last_frame;
                game_controller.lock().unwrap().on_tick(dt);
                this_frame
            };
            game_controller.lock().unwrap().draw(&main);
            window.request_animation_frame(inner_handle.borrow().as_ref().unwrap().as_ref().unchecked_ref());
        }) as Box<dyn FnMut()>));
    }
    window.request_animation_frame(outer_handle.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

fn now() -> f64 {
    js_sys::Date::now() / 1000.0
}
