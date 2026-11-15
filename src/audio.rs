use kira::sound::static_sound::StaticSoundData;
use std::io::Cursor;

#[cfg(not(target_arch = "wasm32"))]
use kira::{
    AudioManager as KiraAudioManager, AudioManagerSettings, Decibels,
    sound::static_sound::StaticSoundHandle,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameSound {
    Ak47,
    Knife,
    Pipe,
    Pistol,
    Swosh,
    Shotgun,
    ShotgunReload,
}

impl GameSound {
    fn slot(self) -> SoundId {
        match self {
            GameSound::Ak47 => SoundId::Ak47,
            GameSound::Knife => SoundId::Knife,
            GameSound::Pipe => SoundId::Pipe,
            GameSound::Pistol => SoundId::Pistol,
            GameSound::Swosh => SoundId::Swosh,
            GameSound::Shotgun => SoundId::Shotgun,
            GameSound::ShotgunReload => SoundId::ShotgunReload,
        }
    }
}

const SOUND_COUNT: usize = 10;
const DEFAULT_VOLUME: f32 = 0.5;

// Замедление как у магнитофона: скорость и высота тона падают вместе.
const SLOW_PLAYBACK_RATE: f64 = 0.4;

const TIME_ENTER_VOLUME: f32 = 0.6;
const TIME_LOOP_VOLUME: f32 = 0.35;
const TIME_LOOP_FADE_IN: f32 = 0.15;
const TIME_LOOP_FADE_OUT: f32 = 0.25;
const TIME_EXIT_VOLUME: f32 = 0.5;

const DUCK_VOLUME: f32 = 0.45;
const DUCK_SPEED: f32 = 6.0;

#[derive(Clone, Copy)]
enum SoundId {
    Ak47 = 0,
    Knife,
    Pipe,
    Pistol,
    Swosh,
    Shotgun,
    ShotgunReload,
    TimeEnter,
    TimeLoop,
    TimeExit,
}

impl SoundId {
    const ALL: [SoundId; SOUND_COUNT] = [
        SoundId::Ak47,
        SoundId::Knife,
        SoundId::Pipe,
        SoundId::Pistol,
        SoundId::Swosh,
        SoundId::Shotgun,
        SoundId::ShotgunReload,
        SoundId::TimeEnter,
        SoundId::TimeLoop,
        SoundId::TimeExit,
    ];

    fn bytes(self) -> &'static [u8] {
        match self {
            SoundId::Ak47 => include_bytes!("../assets/sounds/ak47.ogg"),
            SoundId::Knife => include_bytes!("../assets/sounds/knife.ogg"),
            SoundId::Pipe => include_bytes!("../assets/sounds/pipe.ogg"),
            SoundId::Pistol => include_bytes!("../assets/sounds/pistol.ogg"),
            SoundId::Swosh => include_bytes!("../assets/sounds/swosh.ogg"),
            SoundId::Shotgun => include_bytes!("../assets/sounds/shotgun.ogg"),
            SoundId::ShotgunReload => include_bytes!("../assets/sounds/shotgun_reload.ogg"),
            SoundId::TimeEnter => include_bytes!("../assets/sounds/time_enter.ogg"),
            SoundId::TimeLoop => include_bytes!("../assets/sounds/time_loop.ogg"),
            SoundId::TimeExit => include_bytes!("../assets/sounds/time_exit.ogg"),
        }
    }
}

fn decode(bytes: &'static [u8]) -> StaticSoundData {
    StaticSoundData::from_cursor(Cursor::new(bytes)).expect("не удалось декодировать звук")
}

fn load_sounds() -> [StaticSoundData; SOUND_COUNT] {
    SoundId::ALL.map(|id| decode(id.bytes()))
}

#[cfg(not(target_arch = "wasm32"))]
fn gain_to_db(gain: f32) -> Decibels {
    if gain <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct KiraBackend {
    // Сначала останавливаем луп, потом менеджер: handle не должен пережить канал команд.
    loop_handle: Option<StaticSoundHandle>,
    manager: KiraAudioManager<kira::DefaultBackend>,
    sounds: [StaticSoundData; SOUND_COUNT],
}

#[cfg(not(target_arch = "wasm32"))]
impl KiraBackend {
    fn play(&mut self, id: SoundId, rate: f64, gain: f32) {
        let data = self.sounds[id as usize]
            .clone()
            .playback_rate(rate)
            .volume(gain_to_db(gain));

        if let Err(error) = self.manager.play(data) {
            eprintln!("не удалось проиграть звук: {error}");
        }
    }

    fn set_master_volume(&mut self, volume: f32) {
        self.manager
            .main_track()
            .set_volume(gain_to_db(volume), kira::Tween::default());
    }

    fn start_loop(&mut self, id: SoundId, gain: f32) {
        self.stop_loop();

        let data = self.sounds[id as usize]
            .clone()
            .loop_region(0.0..)
            .volume(gain_to_db(gain));

        match self.manager.play(data) {
            Ok(handle) => self.loop_handle = Some(handle),
            Err(error) => eprintln!("не удалось запустить луп замедления: {error}"),
        }
    }

    fn set_loop_gain(&mut self, gain: f32) {
        if let Some(handle) = &mut self.loop_handle {
            handle.set_volume(gain_to_db(gain), kira::Tween::default());
        }
    }

    fn stop_loop(&mut self) {
        if let Some(mut handle) = self.loop_handle.take() {
            handle.stop(kira::Tween::default());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_kira_backend() -> Option<KiraBackend> {
    let manager = match KiraAudioManager::<kira::DefaultBackend>::new(
        AudioManagerSettings::default(),
    ) {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("звук отключён: не удалось запустить аудиовыход: {error}");
            return None;
        }
    };

    let sounds = load_sounds();

    Some(KiraBackend {
        loop_handle: None,
        manager,
        sounds,
    })
}

#[cfg(target_arch = "wasm32")]
fn load_web_backend() -> Option<crate::web_audio::WebAudio> {
    let sounds = load_sounds();

    crate::web_audio::WebAudio::new(&sounds)
}

enum AudioBackend {
    #[cfg(not(target_arch = "wasm32"))]
    Kira(KiraBackend),
    #[cfg(target_arch = "wasm32")]
    Web(crate::web_audio::WebAudio),
}

impl AudioBackend {
    fn play(&mut self, id: SoundId, rate: f64, gain: f32) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Kira(backend) => backend.play(id, rate, gain),
            #[cfg(target_arch = "wasm32")]
            AudioBackend::Web(backend) => backend.play(id as u32, rate, gain),
        }
    }

    fn set_master_volume(&mut self, volume: f32) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Kira(backend) => backend.set_master_volume(volume),
            #[cfg(target_arch = "wasm32")]
            AudioBackend::Web(backend) => backend.set_master_volume(volume),
        }
    }

    fn start_loop(&mut self, id: SoundId, gain: f32) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Kira(backend) => backend.start_loop(id, gain),
            #[cfg(target_arch = "wasm32")]
            AudioBackend::Web(backend) => backend.start_loop(id as u32, gain),
        }
    }

    fn set_loop_gain(&mut self, gain: f32) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Kira(backend) => backend.set_loop_gain(gain),
            #[cfg(target_arch = "wasm32")]
            AudioBackend::Web(backend) => backend.set_loop_gain(gain),
        }
    }

    fn stop_loop(&mut self) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Kira(backend) => backend.stop_loop(),
            #[cfg(target_arch = "wasm32")]
            AudioBackend::Web(backend) => backend.stop_loop(),
        }
    }
}

fn load_backend() -> Option<AudioBackend> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_kira_backend().map(AudioBackend::Kira)
    }

    #[cfg(target_arch = "wasm32")]
    {
        load_web_backend().map(AudioBackend::Web)
    }
}

pub struct AudioManager {
    volume: f32,
    duck: f32,
    time_loop_playing: bool,
    time_loop_fade: f32,
    slow_playback: bool,
    backend: Option<AudioBackend>,
}

impl AudioManager {
    pub async fn load() -> Self {
        let backend = load_backend();

        let mut manager = Self {
            volume: DEFAULT_VOLUME,
            duck: 1.0,
            time_loop_playing: false,
            time_loop_fade: 0.0,
            slow_playback: false,
            backend,
        };

        manager.apply_volume();
        manager
    }

    pub fn play(&mut self, sound: GameSound) {
        if self.volume <= 0.0 {
            return;
        }

        let gain = self.duck;
        if gain <= 0.0 {
            return;
        }

        let Some(backend) = &mut self.backend else {
            return;
        };

        let playback_rate = if self.slow_playback {
            SLOW_PLAYBACK_RATE
        } else {
            1.0
        };

        backend.play(sound.slot(), playback_rate, gain);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_volume();
    }

    fn apply_volume(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.set_master_volume(self.volume);
        }
    }

    pub fn update(&mut self, real_dt: f32, time_slow_active: bool) {
        self.slow_playback = time_slow_active;

        let duck_target = if time_slow_active { DUCK_VOLUME } else { 1.0 };
        let duck_step = (DUCK_SPEED * real_dt).min(1.0);
        self.duck += (duck_target - self.duck) * duck_step;

        if time_slow_active {
            if !self.time_loop_playing {
                self.time_loop_playing = true;
                self.time_loop_fade = 0.0;

                if let Some(backend) = &mut self.backend {
                    backend.play(SoundId::TimeEnter, 1.0, TIME_ENTER_VOLUME);
                    backend.start_loop(SoundId::TimeLoop, 0.0);
                }
            }

            self.time_loop_fade = (self.time_loop_fade + real_dt / TIME_LOOP_FADE_IN).min(1.0);

            if let Some(backend) = &mut self.backend {
                backend.set_loop_gain(TIME_LOOP_VOLUME * self.time_loop_fade);
            }
        } else if self.time_loop_playing {
            self.time_loop_fade -= real_dt / TIME_LOOP_FADE_OUT;

            if self.time_loop_fade <= 0.0 {
                self.time_loop_playing = false;
                self.time_loop_fade = 0.0;

                if let Some(backend) = &mut self.backend {
                    backend.stop_loop();
                    backend.play(SoundId::TimeExit, 1.0, TIME_EXIT_VOLUME);
                }
            } else if let Some(backend) = &mut self.backend {
                backend.set_loop_gain(TIME_LOOP_VOLUME * self.time_loop_fade);
            }
        }
    }
}
