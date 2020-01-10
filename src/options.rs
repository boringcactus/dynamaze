extern crate toml;

use std::env;
use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Clone, Serialize)]
#[serde(default)]
pub struct GameOptions {
    pub music_level: u8,
    pub sound_level: u8,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            music_level: 50,
            sound_level: 50,
        }
    }
}

trait SuppressError<T> {
    fn suppress_error(self) -> Result<T, ()>;
}

impl<X, Y> SuppressError<X> for Result<X, Y> {
    fn suppress_error(self) -> Result<X, ()> {
        self.map_err(|_| ())
    }
}

pub struct GameOptionsHandle {
    options: RwLock<GameOptions>,
}

fn read() -> Option<String> {
    let window = web_sys::window().unwrap_throw();
    let local_storage = window.local_storage().unwrap_throw().unwrap_throw();
    local_storage.get_item("settings").unwrap_throw()
}

fn write(settings: &str) {
    let window = web_sys::window().unwrap_throw();
    let local_storage = window.local_storage().unwrap_throw().unwrap_throw();
    local_storage.set_item("settings", settings);
}

impl GameOptionsHandle {
    fn new() -> Self {
        let options = read()
            .and_then(|x| toml::from_str(&x).ok())
            .unwrap_or_default();
        GameOptionsHandle {
            options: RwLock::new(options),
        }
    }

    pub fn fetch(&self) -> RwLockReadGuard<GameOptions> {
        self.options.read().unwrap()
    }

    pub fn save(&self, options: &GameOptions) {
        *(self.options.write().unwrap()) = options.clone();
        let _ = toml::to_string_pretty(options).suppress_error()
            .map(|data| write(&data));
    }
}

lazy_static! {
    pub static ref HANDLE: GameOptionsHandle = {
        GameOptionsHandle::new()
    };
}
