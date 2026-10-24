#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

SLOW="${1:-0.4}"

mkdir -p assets/sounds/slow

for file in assets/sounds/*.wav; do
    name="$(basename "$file")"

    case "$name" in
        time_*) continue ;;
    esac

    rate="$(ffprobe -v error -select_streams a:0 -show_entries stream=sample_rate -of csv=p=0 "$file")"
    rate_slow="$(awk -v r="$rate" -v f="$SLOW" 'BEGIN { printf "%.0f", r * f }')"

    ffmpeg -hide_banner -loglevel error -y -i "$file" \
        -af "asetrate=${rate_slow},aresample=${rate}" \
        -c:a pcm_s16le "assets/sounds/slow/${name}"

    echo "assets/sounds/slow/${name} (${rate} -> ${rate_slow} Hz)"
done

echo "готово"
