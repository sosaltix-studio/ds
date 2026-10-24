use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
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
    const ALL: [GameSound; 7] = [
        GameSound::Ak47,
        GameSound::Knife,
        GameSound::Pipe,
        GameSound::Pistol,
        GameSound::Swosh,
        GameSound::Shotgun,
        GameSound::ShotgunReload,
    ];

    fn normal_bytes(self) -> &'static [u8] {
        match self {
            GameSound::Ak47 => include_bytes!("../assets/sounds/ak47.wav"),
            GameSound::Knife => include_bytes!("../assets/sounds/knife.wav"),
            GameSound::Pipe => include_bytes!("../assets/sounds/pipe.wav"),
            GameSound::Pistol => include_bytes!("../assets/sounds/pistol.wav"),
            GameSound::Swosh => include_bytes!("../assets/sounds/swosh.wav"),
            GameSound::Shotgun => include_bytes!("../assets/sounds/shotgun.wav"),
            GameSound::ShotgunReload => include_bytes!("../assets/sounds/shotgun_reload.wav"),
        }
    }

    fn slow_bytes(self) -> &'static [u8] {
        match self {
            GameSound::Ak47 => include_bytes!("../assets/sounds/slow/ak47.wav"),
            GameSound::Knife => include_bytes!("../assets/sounds/slow/knife.wav"),
            GameSound::Pipe => include_bytes!("../assets/sounds/slow/pipe.wav"),
            GameSound::Pistol => include_bytes!("../assets/sounds/slow/pistol.wav"),
            GameSound::Swosh => include_bytes!("../assets/sounds/slow/swosh.wav"),
            GameSound::Shotgun => include_bytes!("../assets/sounds/slow/shotgun.wav"),
            GameSound::ShotgunReload => include_bytes!("../assets/sounds/slow/shotgun_reload.wav"),
        }
    }
}

const SOUND_COUNT: usize = GameSound::ALL.len();

const TIME_ENTER_VOLUME: f32 = 0.6;
const TIME_LOOP_VOLUME: f32 = 0.35;
const TIME_LOOP_FADE_IN: f32 = 0.15;
const TIME_LOOP_FADE_OUT: f32 = 0.25;
const TIME_EXIT_VOLUME: f32 = 0.5;

const DUCK_VOLUME: f32 = 0.45;
const DUCK_SPEED: f32 = 6.0;

pub struct AudioManager {
    sounds: [Sound; SOUND_COUNT],
    slow_sounds: [Sound; SOUND_COUNT],
    pub sound_time_enter: Sound,
    pub sound_time_loop: Sound,
    pub sound_time_exit: Sound,
    pub volume: f32,
    duck: f32,
    slow_playback: bool,
    time_loop_playing: bool,
    time_loop_fade: f32,
}

impl AudioManager {
    pub async fn load() -> Self {
        let mut sounds = Vec::with_capacity(SOUND_COUNT);
        let mut slow_sounds = Vec::with_capacity(SOUND_COUNT);
        for sound in GameSound::ALL {
            sounds.push(load_sound_from_bytes(sound.normal_bytes()).await.unwrap());
            slow_sounds.push(load_sound_from_bytes(sound.slow_bytes()).await.unwrap());
        }

        let sound_time_enter =
            load_sound_from_bytes(include_bytes!("../assets/sounds/time_enter.wav"))
                .await
                .unwrap();
        let sound_time_loop =
            load_sound_from_bytes(include_bytes!("../assets/sounds/time_loop.wav"))
                .await
                .unwrap();
        let sound_time_exit =
            load_sound_from_bytes(include_bytes!("../assets/sounds/time_exit.wav"))
                .await
                .unwrap();

        Self {
            sounds: sounds.try_into().unwrap(),
            slow_sounds: slow_sounds.try_into().unwrap(),
            sound_time_enter,
            sound_time_loop,
            sound_time_exit,
            volume: 0.5,
            duck: 1.0,
            slow_playback: false,
            time_loop_playing: false,
            time_loop_fade: 0.0,
        }
    }

    pub fn play(&self, sound: GameSound) {
        let source = if self.slow_playback {
            &self.slow_sounds[sound as usize]
        } else {
            &self.sounds[sound as usize]
        };

        let volume = self.volume * self.duck;
        if volume > 0.0 {
            play_sound(
                source,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
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

                play_sound(
                    &self.sound_time_enter,
                    PlaySoundParams {
                        looped: false,
                        volume: TIME_ENTER_VOLUME * self.volume,
                    },
                );
                play_sound(
                    &self.sound_time_loop,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.0,
                    },
                );
            }

            self.time_loop_fade = (self.time_loop_fade + real_dt / TIME_LOOP_FADE_IN).min(1.0);
            set_sound_volume(
                &self.sound_time_loop,
                TIME_LOOP_VOLUME * self.time_loop_fade * self.volume,
            );
        } else if self.time_loop_playing {
            self.time_loop_fade -= real_dt / TIME_LOOP_FADE_OUT;

            if self.time_loop_fade <= 0.0 {
                self.time_loop_playing = false;
                self.time_loop_fade = 0.0;
                stop_sound(&self.sound_time_loop);
                play_sound(
                    &self.sound_time_exit,
                    PlaySoundParams {
                        looped: false,
                        volume: TIME_EXIT_VOLUME * self.volume,
                    },
                );
            } else {
                set_sound_volume(
                    &self.sound_time_loop,
                    TIME_LOOP_VOLUME * self.time_loop_fade * self.volume,
                );
            }
        }
    }
}
