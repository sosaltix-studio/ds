#!/usr/bin/env bash
# Кодирование исходных wav-ассетов в ogg vorbis, которые встраиваются в бинарник.
set -euo pipefail

cd "$(dirname "$0")/.."

for file in assets/sounds/*.wav; do
    name="$(basename "${file%.wav}")"

    ffmpeg -hide_banner -loglevel error -y -i "$file" \
        -ar 44100 -c:a libvorbis -q:a 4 "assets/sounds/${name}.ogg"

    echo "assets/sounds/${name}.ogg"
done

echo "готово"
