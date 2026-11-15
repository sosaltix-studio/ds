#[cfg(not(target_arch = "wasm32"))]
use kira::{
    AudioManager as KiraAudioManager, AudioManagerSettings, Decibels, DefaultBackend,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Cursor;

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

#[cfg(not(target_arch = "wasm32"))]
impl GameSound {
    const ALL: [GameSound; 7] = [
        GameSound::Ak47,
        GameSound::Knife,
        GameSound::Pipe,
        GameSound::Pistol,
        GameSound::Swosh,
        GameSound::Shotgun,
        GameSound::ShotgunReload,
    ];

    fn bytes(self) -> &'static [u8] {
        match self {
            GameSound::Ak47 => include_bytes!("../assets/sounds/ak47.ogg"),
            GameSound::Knife => include_bytes!("../assets/sounds/knife.ogg"),
            GameSound::Pipe => include_bytes!("../assets/sounds/pipe.ogg"),
            GameSound::Pistol => include_bytes!("../assets/sounds/pistol.ogg"),
            GameSound::Swosh => include_bytes!("../assets/sounds/swosh.ogg"),
            GameSound::Shotgun => include_bytes!("../assets/sounds/shotgun.ogg"),
            GameSound::ShotgunReload => include_bytes!("../assets/sounds/shotgun_reload.ogg"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
const SOUND_COUNT: usize = GameSound::ALL.len();

#[cfg(not(target_arch = "wasm32"))]
const SLOW_PLAYBACK_RATE: f64 = 0.4;

#[cfg(not(target_arch = "wasm32"))]
const TIME_ENTER_VOLUME: f32 = 0.6;
#[cfg(not(target_arch = "wasm32"))]
const TIME_LOOP_VOLUME: f32 = 0.35;
const TIME_LOOP_FADE_IN: f32 = 0.15;
const TIME_LOOP_FADE_OUT: f32 = 0.25;
#[cfg(not(target_arch = "wasm32"))]
const TIME_EXIT_VOLUME: f32 = 0.5;

const DUCK_VOLUME: f32 = 0.45;
const DUCK_SPEED: f32 = 6.0;

#[cfg(not(target_arch = "wasm32"))]
struct DesktopAudio {
    manager: KiraAudioManager<DefaultBackend>,
    sounds: [StaticSoundData; SOUND_COUNT],
    time_enter: StaticSoundData,
    time_loop: StaticSoundData,
    time_exit: StaticSoundData,
    time_loop_handle: Option<StaticSoundHandle>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopAudio {
    fn load() -> Result<Self, String> {
        let manager = KiraAudioManager::new(AudioManagerSettings::default())
            .map_err(|error| format!("не удалось запустить аудиовыход: {error}"))?;

        let sounds = std::array::from_fn(|index| decode(GameSound::ALL[index].bytes()));

        Ok(Self {
            manager,
            sounds,
            time_enter: decode(include_bytes!("../assets/sounds/time_enter.ogg")),
            time_loop: decode(include_bytes!("../assets/sounds/time_loop.ogg")),
            time_exit: decode(include_bytes!("../assets/sounds/time_exit.ogg")),
            time_loop_handle: None,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn decode(bytes: &'static [u8]) -> StaticSoundData {
    StaticSoundData::from_cursor(Cursor::new(bytes)).expect("не удалось декодировать звук")
}

#[cfg(not(target_arch = "wasm32"))]
fn gain_to_db(gain: f32) -> Decibels {
    if gain <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

pub struct AudioManager {
    pub volume: f32,
    duck: f32,
    time_loop_playing: bool,
    time_loop_fade: f32,
    #[cfg(not(target_arch = "wasm32"))]
    slow_playback: bool,
    #[cfg(not(target_arch = "wasm32"))]
    desktop: Option<DesktopAudio>,
}

impl AudioManager {
    pub async fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let desktop = match DesktopAudio::load() {
            Ok(desktop) => Some(desktop),
            Err(error) => {
                eprintln!("звук отключён: {error}");
                None
            }
        };

        Self {
            volume: 0.5,
            duck: 1.0,
            time_loop_playing: false,
            time_loop_fade: 0.0,
            #[cfg(not(target_arch = "wasm32"))]
            slow_playback: false,
            #[cfg(not(target_arch = "wasm32"))]
            desktop,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn play(&mut self, sound: GameSound) {
        let gain = self.volume * self.duck;
        if gain <= 0.0 {
            return;
        }

        let Some(desktop) = &mut self.desktop else {
            return;
        };

        let playback_rate = if self.slow_playback {
            SLOW_PLAYBACK_RATE
        } else {
            1.0
        };

        let data = desktop.sounds[sound as usize]
            .clone()
            .playback_rate(playback_rate)
            .volume(gain_to_db(gain));

        if let Err(error) = desktop.manager.play(data) {
            eprintln!("не удалось проиграть звук: {error}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn play(&mut self, _sound: GameSound) {
        // В браузерной сборке звук пока отключён.
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn update(&mut self, real_dt: f32, time_slow_active: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.slow_playback = time_slow_active;
        }

        let duck_target = if time_slow_active { DUCK_VOLUME } else { 1.0 };
        let duck_step = (DUCK_SPEED * real_dt).min(1.0);
        self.duck += (duck_target - self.duck) * duck_step;

        if time_slow_active {
            if !self.time_loop_playing {
                self.time_loop_playing = true;
                self.time_loop_fade = 0.0;

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(desktop) = &mut self.desktop {
                    let enter = desktop
                        .time_enter
                        .clone()
                        .volume(gain_to_db(TIME_ENTER_VOLUME * self.volume));
                    if let Err(error) = desktop.manager.play(enter) {
                        eprintln!("не удалось проиграть звук входа в замедление: {error}");
                    }

                    let looped = desktop
                        .time_loop
                        .clone()
                        .loop_region(0.0..)
                        .volume(Decibels::SILENCE);
                    match desktop.manager.play(looped) {
                        Ok(handle) => desktop.time_loop_handle = Some(handle),
                        Err(error) => {
                            eprintln!("не удалось запустить луп замедления: {error}")
                        }
                    }
                }
            }

            self.time_loop_fade = (self.time_loop_fade + real_dt / TIME_LOOP_FADE_IN).min(1.0);

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(desktop) = &mut self.desktop {
                if let Some(handle) = &mut desktop.time_loop_handle {
                    handle.set_volume(
                        gain_to_db(TIME_LOOP_VOLUME * self.time_loop_fade * self.volume),
                        kira::Tween::default(),
                    );
                }
            }
        } else if self.time_loop_playing {
            self.time_loop_fade -= real_dt / TIME_LOOP_FADE_OUT;

            if self.time_loop_fade <= 0.0 {
                self.time_loop_playing = false;
                self.time_loop_fade = 0.0;

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(desktop) = &mut self.desktop {
                    if let Some(mut handle) = desktop.time_loop_handle.take() {
                        handle.stop(kira::Tween::default());
                    }

                    let exit = desktop
                        .time_exit
                        .clone()
                        .volume(gain_to_db(TIME_EXIT_VOLUME * self.volume));
                    if let Err(error) = desktop.manager.play(exit) {
                        eprintln!("не удалось проиграть звук выхода из замедления: {error}");
                    }
                }
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(desktop) = &mut self.desktop {
                    if let Some(handle) = &mut desktop.time_loop_handle {
                        handle.set_volume(
                            gain_to_db(TIME_LOOP_VOLUME * self.time_loop_fade * self.volume),
                            kira::Tween::default(),
                        );
                    }
                }
            }
        }
    }
}
