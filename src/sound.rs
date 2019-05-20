use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use std::time::Duration;

use rodio::{self, *};

use crate::options;

const MUSIC_VOLUME: f32 = 0.6;
const SOUND_VOLUME: f32 = 0.4;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Music {
    Menu,
    InGame,
}

impl Into<File> for Music {
    fn into(self) -> File {
        match self {
            Music::Menu => File::open("assets/BlueEther.mp3").expect("Failed to open audio"),
            Music::InGame => File::open("assets/ElectricSweater.mp3").expect("Failed to open audio"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Sound {
    YourTurn,
}

impl Sound {
    fn play_on(self, sink: &Sink) {
        match self {
            Sound::YourTurn => {
                let freqs = [349, 440, 523, 698];
                for freq in &freqs {
                    let source = rodio::source::SineWave::new(*freq);
                    let source = source.fade_in(Duration::from_millis(10));
                    sink.append(source.take_duration(Duration::from_millis(100)));
                }
            }
        }
    }
}

pub struct SoundEngine {
    device: Device,
    music_sinks: Mutex<HashMap<Music, Sink>>,
    sound_sinks: Mutex<HashMap<Sound, Sink>>,
    current_music: Mutex<Option<Music>>,
}

impl SoundEngine {
    fn new() -> SoundEngine {
        let device = rodio::default_output_device().unwrap();
        SoundEngine {
            device,
            music_sinks: Mutex::new(HashMap::new()),
            sound_sinks: Mutex::new(HashMap::new()),
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
        let mut music_sinks = self.music_sinks.lock().unwrap();
        let mut current_music = self.current_music.lock().unwrap();
        if let Some(ref old_music) = *current_music {
            if let Some(old_sink) = music_sinks.get(old_music) {
                old_sink.pause();
            }
        }
        let sink = music_sinks.entry(music).or_insert_with(|| {
            let mut sink = Sink::new(&self.device);
            sink.set_volume(MUSIC_VOLUME);
            let file: File = music.into();
            let source = Decoder::new(BufReader::new(file)).expect("Failed to decode");
            sink.pause();
            sink.append(source.repeat_infinite());
            sink
        });
        sink.set_volume(MUSIC_VOLUME * (f32::from(options::HANDLE.fetch().audio_level)) / 100.0);
        sink.play();
        *current_music = Some(music);
    }

    pub fn fetch_volume(&self) {
        let current_music = self.current_music.lock().unwrap();
        if let Some(music) = *current_music {
            let mut music_sinks = self.music_sinks.lock().unwrap();
            if let Some(sink) = music_sinks.get_mut(&music) {
                sink.set_volume(MUSIC_VOLUME * (f32::from(options::HANDLE.fetch().audio_level)) / 100.0);
            }
        }
    }

    pub fn poke_volume(&self, audio_level: u8) {
        let current_music = self.current_music.lock().unwrap();
        if let Some(music) = *current_music {
            let mut music_sinks = self.music_sinks.lock().unwrap();
            if let Some(sink) = music_sinks.get_mut(&music) {
                sink.set_volume(MUSIC_VOLUME * (f32::from(audio_level)) / 100.0);
            }
        }
    }

    pub fn play_sound(&self, snd: Sound) {
        let mut sound_sinks = self.sound_sinks.lock().unwrap();
        let sink = sound_sinks.entry(snd).or_insert_with(|| {
            let mut sink = Sink::new(&self.device);
            sink.set_volume(SOUND_VOLUME);
            sink
        });
        sink.set_volume(SOUND_VOLUME * (f32::from(options::HANDLE.fetch().audio_level)) / 100.0);
        snd.play_on(sink);
    }
}

lazy_static! {
    pub static ref SOUND: SoundEngine = {
        SoundEngine::new()
    };
}
