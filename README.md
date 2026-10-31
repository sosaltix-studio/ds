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
