//! Вывод звука в браузере.
//!
//! Kira на wasm используется только как декодер OGG: готовый PCM один раз
//! передаётся в `web_audio.js`, а проигрывание, громкость, зацикливание и
//! изменение скорости делает штатный WebAudio-граф браузера.
//!
//! Так мы не тянем сэмплы из wasm через `ScriptProcessorNode` и не микшируем
//! звук в главном потоке игры. OGG декодируется внутри wasm, поэтому звук не
//! зависит от того, поддерживает ли браузер Ogg Vorbis.

use kira::sound::static_sound::StaticSoundData;

unsafe extern "C" {
    /// Создаёт `AudioContext` и возвращает его частоту дискретизации.
    /// При ошибке возвращает `0`.
    fn ds_audio_init() -> f64;

    /// Разрешает вывод после первого действия пользователя.
    fn ds_audio_start();

    /// Загружает PCM (interleaved) в браузер. Возвращает 1 при успехе.
    fn ds_audio_upload(
        id: u32,
        samples: *const f32,
        frames: u32,
        channels: u32,
        sample_rate: f64,
    ) -> u32;

    /// Играет один звук: `id`, скорость, линейная громкость.
    fn ds_audio_play(id: u32, rate: f64, gain: f32);

    /// Запускает зацикленный звук с начальной громкостью.
    fn ds_audio_loop_start(id: u32, gain: f32);

    /// Меняет громкость текущего лупа.
    fn ds_audio_loop_gain(gain: f32);

    /// Останавливает текущий луп.
    fn ds_audio_loop_stop();

    /// Громкость всего микса.
    fn ds_audio_set_master(gain: f32);

    /// Останавливает всё и закрывает `AudioContext`.
    fn ds_audio_shutdown();
}

pub struct WebAudio;

impl WebAudio {
    pub fn new(sounds: &[StaticSoundData]) -> Option<Self> {
        let sample_rate = unsafe { ds_audio_init() };
        if !sample_rate.is_finite() || sample_rate < 8000.0 {
            return None;
        }

        for (id, sound) in sounds.iter().enumerate() {
            upload(id, sound);
        }

        unsafe { ds_audio_start() };

        Some(Self)
    }

    pub fn play(&mut self, id: u32, rate: f64, gain: f32) {
        unsafe { ds_audio_play(id, rate, gain) };
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        unsafe { ds_audio_set_master(volume) };
    }

    pub fn start_loop(&mut self, id: u32, gain: f32) {
        unsafe { ds_audio_loop_start(id, gain) };
    }

    pub fn set_loop_gain(&mut self, gain: f32) {
        unsafe { ds_audio_loop_gain(gain) };
    }

    pub fn stop_loop(&mut self) {
        unsafe { ds_audio_loop_stop() };
    }
}

impl Drop for WebAudio {
    fn drop(&mut self) {
        unsafe { ds_audio_shutdown() };
    }
}

fn upload(id: usize, sound: &StaticSoundData) {
    let frames = sound.frames.len();
    if frames == 0 {
        eprintln!("звук {id} пустой");
        return;
    }

    // Kira хранит стерео-фреймы; WebAudio ждёт interleaved PCM.
    let mut pcm = Vec::with_capacity(frames * 2);
    for frame in sound.frames.iter() {
        pcm.push(frame.left);
        pcm.push(frame.right);
    }

    let uploaded = unsafe {
        ds_audio_upload(
            id as u32,
            pcm.as_ptr(),
            frames as u32,
            2,
            sound.sample_rate as f64,
        )
    };

    if uploaded == 0 {
        eprintln!("не удалось загрузить звук {id} в WebAudio");
    }
}

/// Версия JS-плагина, которую проверяет загрузчик miniquad.
#[unsafe(no_mangle)]
pub extern "C" fn ds_audio_crate_version() -> u32 {
    2
}
