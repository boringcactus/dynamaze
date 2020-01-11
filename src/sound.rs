use std::collections::HashMap;
use std::sync::Mutex;

use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, GainNode, HtmlAudioElement};

use crate::options;

const MUSIC_VOLUME: f32 = 0.6;
const SOUND_VOLUME: f32 = 0.4;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Music {
    Menu,
    InGame,
}

impl Music {
    fn load(self) -> HtmlAudioElement {
        let path = match self {
            Music::Menu => "assets/BlueEther.mp3",
            Music::InGame => "assets/ElectricSweater.mp3",
        };

        let result = HtmlAudioElement::new_with_src(path).unwrap_throw();
        result.set_loop(true);
        result
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Sound {
    YourTurn,
}

impl Sound {
    fn load(self) -> HtmlAudioElement {
        let path = match self {
            Sound::YourTurn => "assets/TurnPing.wav",
        };

        HtmlAudioElement::new_with_src(path).unwrap_throw()
    }
}

pub struct SoundEngine {
    context: AudioContext,
    music_sources: Mutex<HashMap<Music, HtmlAudioElement>>,
    sound_sources: Mutex<HashMap<Sound, HtmlAudioElement>>,
    music_gain: GainNode,
    sound_gain: GainNode,
    current_music: Mutex<Option<Music>>,
}

impl SoundEngine {
    pub fn new() -> SoundEngine {
        let context = AudioContext::new().unwrap_throw();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("started sound engine"));
        let music_gain = context
            .create_gain()
            .expect_throw("Failed to create music gain node");
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("yeet sound engine"));
        music_gain
            .gain()
            .set_value(MUSIC_VOLUME * (f32::from(options::HANDLE.fetch().music_level)) / 100.0);
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("swag sound engine"));
        music_gain
            .connect_with_audio_node(&context.destination())
            .unwrap_throw();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("made sound engine"));
        let sound_gain = context
            .create_gain()
            .expect_throw("Failed to create sound  gain node");
        sound_gain
            .gain()
            .set_value(SOUND_VOLUME * (f32::from(options::HANDLE.fetch().sound_level)) / 100.0);
        sound_gain
            .connect_with_audio_node(&context.destination())
            .unwrap_throw();
        SoundEngine {
            context,
            music_sources: Mutex::new(HashMap::new()),
            sound_sources: Mutex::new(HashMap::new()),
            music_gain,
            sound_gain,
            current_music: Mutex::new(None),
        }
    }

    pub fn play_music(&self, music: Music) {
        {
            let current_music = self.current_music.lock().unwrap();
            if *current_music == Some(music) {
                return;
            }
        }
        let _ = self.context.resume();
        let mut music_sources = self.music_sources.lock().unwrap();
        let mut current_music = self.current_music.lock().unwrap();
        if let Some(ref old_music) = *current_music {
            if let Some(old_source) = music_sources.get(old_music) {
                old_source.pause();
            }
        }
        let source = music_sources.entry(music).or_insert_with(|| {
            let source = music.load();
            let source_node = self
                .context
                .create_media_element_source(&source)
                .unwrap_throw();
            source_node
                .connect_with_audio_node(&self.music_gain)
                .unwrap_throw();
            source
        });
        source.play();
        *current_music = Some(music);
    }

    pub fn play_sound(&self, snd: Sound) {
        let _ = self.context.resume();
        let mut sound_sources = self.sound_sources.lock().unwrap();
        let source = sound_sources.entry(snd).or_insert_with(|| {
            let source = snd.load();
            let source_node = self
                .context
                .create_media_element_source(&source)
                .unwrap_throw();
            source_node
                .connect_with_audio_node(&self.sound_gain)
                .unwrap_throw();
            source
        });
        source.play();
    }

    pub fn fetch_volume(&self) {
        self.music_gain
            .gain()
            .set_value(MUSIC_VOLUME * (f32::from(options::HANDLE.fetch().music_level)) / 100.0);
        self.sound_gain
            .gain()
            .set_value(SOUND_VOLUME * (f32::from(options::HANDLE.fetch().sound_level)) / 100.0);
    }

    pub fn poke_options(&self, new_options: &options::GameOptions) {
        self.music_gain
            .gain()
            .set_value(MUSIC_VOLUME * (f32::from(new_options.music_level)) / 100.0);
        self.sound_gain
            .gain()
            .set_value(SOUND_VOLUME * (f32::from(new_options.sound_level)) / 100.0);
    }
}

impl Default for SoundEngine {
    fn default() -> Self {
        Self::new()
    }
}
