extern crate serde_piecewise_default;
extern crate toml;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard};

use serde::{Deserialize, Serialize};
use serde_piecewise_default::DeserializePiecewiseDefault;

#[derive(DeserializePiecewiseDefault, Clone, Serialize)]
pub struct GameOptions {
    pub audio_level: u8,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            audio_level: 50,
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

lazy_static! {
    static ref CONFIG_PATH: PathBuf = {
        let mut config = env::current_exe().unwrap();
        config.set_extension("toml");
        config
    };
}

impl GameOptionsHandle {
    fn new() -> Self {
        let options = fs::read(&*CONFIG_PATH)
            .suppress_error()
            .and_then(|x| toml::from_slice(&x).suppress_error())
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
            .and_then(|data| fs::write(&*CONFIG_PATH, data).suppress_error());
    }
}

lazy_static! {
    pub static ref HANDLE: GameOptionsHandle = {
        GameOptionsHandle::new()
    };
}
