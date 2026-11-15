# DS

DS (Dolbaeb Simulator) - игра написанная на Rust (macroquad)

### Лицензия

Проект распространяется под лицензией GNU General Public License v3.0.
Полный текст лицензии — в [LICENSE.md](LICENSE.md).

### Требования
Для сборки проекта вам понадобится установленный компилятор Rust (`rustc` и менеджер пакетов `cargo`). Если у вас их нет, установите с официального сайта [rustup.rs](https://rustup.rs/).

### Установка и запуск

   ```bash
   git clone https://github.com/sosaltix-studio/ds

   cd ds
   
   cargo run --release
   ```
   
### Запуск в браузере

   ```bash
   git clone https://github.com/sosaltix-studio/ds

   cd ds

   rustup target add wasm32-unknown-unknown

   cargo build --release --target wasm32-unknown-unknown

   cp target/wasm32-unknown-unknown/release/ds.wasm .
   
   python -m http.server

   ```
Игра будет доступна здесь http://localhost:8000/

### Звук

И на десктопе, и в браузере используется [kira](https://github.com/tesselode/kira). Замедление времени меняет скорость и высоту тона звуков (эффект «магнитофона»), поэтому отдельные замедленные файлы не нужны.

На десктопе kira выводит звук через `cpal`. В браузере штатный wasm-бэкенд kira требует `wasm-bindgen`, который несовместим с загрузчиком miniquad, поэтому используется собственный минимальный бэкенд: `web_audio.js` создаёт `AudioContext` и тянет сэмплы из wasm через `ScriptProcessorNode`. Файл `web_audio.js` должен лежать рядом с `index.html` и `ds.wasm` (при локальном запуске он уже там, CI копирует его в `dist/` при деплое). Браузеры запускают звук только после первого клика или нажатия клавиши. Safari не поддерживает OGG Vorbis, поэтому в нём звука не будет.

Звуки встраиваются в бинарник в формате OGG Vorbis. Исходные WAV-файлы лежат рядом; после их изменения перекодировать ассеты можно так:

   ```bash
   ./scripts/encode_sounds.sh
   ```

