#!/usr/bin/env bash
# Генерация wav-ассетов для звукового сопровождения замедления времени.
# Все звуки синтезируются из простых формул через ffmpeg (aevalsrc).
set -euo pipefail

cd "$(dirname "$0")/.."

FFMPEG="ffmpeg -hide_banner -loglevel error -y"

# Вход: сабвуферный дроп + свистящий шум
$FFMPEG -f lavfi -i "aevalsrc='0.55*sin(2*PI*170*pow(0.2,t/0.7)*t)*(1-t/0.7)+0.30*(random(0)-0.5)*pow(1-t/0.7,1.5)':d=0.7:s=44100:c=mono" \
    -af "lowpass=f=1400" -c:a pcm_s16le -ar 44100 -ac 1 assets/sounds/time_enter.wav

# Луп: низкий дрон с медленным биением (длины подобраны так, чтобы шов был бесшовным)
$FFMPEG -f lavfi -i "aevalsrc='0.36*sin(2*PI*55*t)*(0.75+0.25*sin(2*PI*t/3))+0.20*sin(2*PI*110*t)*(0.75+0.25*sin(2*PI*2*t/3))+0.10*sin(2*PI*165*t)+0.26*sin(2*PI*(166/3)*t)':d=3:s=44100:c=mono" \
    -c:a pcm_s16le -ar 44100 -ac 1 assets/sounds/time_loop.wav

# Выход: обратный (восходящий) свист + короткий стеклянный "пинг"
$FFMPEG -f lavfi -i "aevalsrc='0.45*sin(2*PI*130*pow(9,t/0.5)*t)*(1-t/0.5)+0.25*sin(2*PI*1250*t)*exp(-9*t)+0.18*sin(2*PI*1875*t)*exp(-12*t)':d=0.5:s=44100:c=mono" \
    -c:a pcm_s16le -ar 44100 -ac 1 assets/sounds/time_exit.wav

echo "готово"
