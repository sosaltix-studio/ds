//! Минимальный WebAudio-бэкенд для kira.
//!
//! Штатный бэкенд kira для wasm построен на `wasm-bindgen` и несовместим с
//! загрузчиком miniquad, который инстанцирует «сырой» wasm-модуль. Поэтому
//! здесь kira подключается к `ScriptProcessorNode` из `web_audio.js`:
//! браузер вызывает [`ds_audio_render`], а тот рендерит очередной блок сэмплов.

use std::cell::RefCell;
use std::fmt;

use kira::backend::{Backend, Renderer};

const CHANNELS: u16 = 2;
const MAX_FRAMES: usize = 2048;

thread_local! {
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

/// Буфер, в который kira пишет готовый стерео-микс.
///
/// Доступ к нему идёт только с главного потока wasm: `ScriptProcessorNode`
/// вызывает `onaudioprocess` на том же потоке, что и игровой цикл, поэтому
/// гонок здесь нет.
struct RenderBuffer(std::cell::UnsafeCell<[f32; MAX_FRAMES * CHANNELS as usize]>);

// SAFETY: wasm без shared memory однопоточный, а доступ к буферу всегда
// происходит на главном потоке.
unsafe impl Sync for RenderBuffer {}

static RENDER_BUFFER: RenderBuffer = RenderBuffer(std::cell::UnsafeCell::new(
    [0.0; MAX_FRAMES * CHANNELS as usize],
));

unsafe extern "C" {
    /// Создаёт `AudioContext` и возвращает его частоту дискретизации.
    /// При ошибке возвращает `0`.
    fn ds_audio_init() -> f64;

    /// Подключает `ScriptProcessorNode` к выходу.
    fn ds_audio_start();
}

/// Ошибка инициализации WebAudio.
#[derive(Debug, Clone, Copy)]
pub struct WasmBackendError;

impl fmt::Display for WasmBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("не удалось инициализировать WebAudio")
    }
}

impl std::error::Error for WasmBackendError {}

/// Настройки WebAudio-бэкенда.
#[derive(Debug, Clone, Copy, Default)]
pub struct WasmBackendSettings;

/// Бэкенд kira поверх WebAudio.
pub struct WasmBackend;

impl Backend for WasmBackend {
    type Settings = WasmBackendSettings;
    type Error = WasmBackendError;

    fn setup(
        _settings: Self::Settings,
        _internal_buffer_size: usize,
    ) -> Result<(Self, u32), Self::Error> {
        let sample_rate = unsafe { ds_audio_init() };
        if !sample_rate.is_finite() || sample_rate < 8000.0 {
            return Err(WasmBackendError);
        }

        Ok((Self, sample_rate as u32))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error> {
        RENDERER.with(|cell| {
            *cell.borrow_mut() = Some(renderer);
        });

        unsafe { ds_audio_start() };
        Ok(())
    }
}

/// Указатель на буфер с последним миксом (interleaved stereo).
#[unsafe(no_mangle)]
pub extern "C" fn ds_audio_buffer_ptr() -> *const f32 {
    RENDER_BUFFER.0.get() as *const f32
}

/// Рендерит `frames` стерео-фреймов в [`RENDER_BUFFER`].
#[unsafe(no_mangle)]
pub extern "C" fn ds_audio_render(frames: u32) {
    let frames = (frames as usize).min(MAX_FRAMES);
    let sample_count = frames * CHANNELS as usize;

    // SAFETY: см. комментарий у `RenderBuffer`.
    let out = unsafe { &mut (&mut *RENDER_BUFFER.0.get())[..sample_count] };

    RENDERER.with(|cell| {
        let mut renderer = cell.borrow_mut();
        match renderer.as_mut() {
            Some(renderer) => {
                renderer.on_start_processing();
                renderer.process(out, CHANNELS);
            }
            None => out.fill(0.0),
        }
    });
}

/// Версия JS-плагина, которую проверяет загрузчик miniquad.
#[unsafe(no_mangle)]
pub extern "C" fn ds_audio_crate_version() -> u32 {
    1
}
