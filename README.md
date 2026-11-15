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

Звуки декодируются из OGG Vorbis библиотекой [kira](https://github.com/tesselode/kira). Замедление времени меняет скорость и высоту тона звуков (эффект «магнитофона»), поэтому отдельные замедленные файлы не нужны.

На десктопе kira выводит звук через `cpal`. В браузере kira используется только как декодер: готовый PCM один раз передаётся в `web_audio.js`, а проигрывание, громкость, зацикливание и изменение скорости выполняет штатный WebAudio-граф браузера (`AudioBufferSourceNode` + `GainNode`). Поэтому в браузерной сборке нет `ScriptProcessorNode`, а OGG не обязан поддерживаться самим браузером — Safari тоже работает. Файл `web_audio.js` должен лежать рядом с `index.html` и `ds.wasm` (при локальном запуске он уже там, CI копирует его в `dist/` при деплое). Браузеры запускают звук только после первого клика или нажатия клавиши.

Звуки встраиваются в бинарник в формате OGG Vorbis. Исходные WAV-файлы лежат рядом; после их изменения перекодировать ассеты можно так:

   ```bash
   ./scripts/encode_sounds.sh
   ```

