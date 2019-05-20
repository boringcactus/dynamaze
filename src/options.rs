use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Serialize)]
pub struct GameOptions {
    pub audio_level: u8,
}

#[derive(Deserialize)]
struct MaybeGameOptions {
    audio_level: Option<u8>,
}

impl Default for MaybeGameOptions {
    fn default() -> Self {
        MaybeGameOptions {
            audio_level: None,
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
        let maybe_options: MaybeGameOptions = fs::read(&*CONFIG_PATH)
            .suppress_error()
            .and_then(|x| toml::from_slice(&x).suppress_error())
            .unwrap_or_default();
        let audio_level = maybe_options.audio_level.unwrap_or(50);
        let options = GameOptions {
            audio_level,
        };
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
