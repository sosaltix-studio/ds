#version 100
precision mediump float;

varying vec2 uv;
varying vec4 color;

uniform sampler2D Texture;
uniform float u_time;
uniform float u_intensity;
uniform float u_glitch;

// Псевдослучайная функция (как в da.glsl)
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 sample_uv = uv;

    // --- ГЛИТЧ-БЕРСТ (только вход/выход из замедления) ---
    // Горизонтальные срезы со случайным сдвигом
    float band = floor(uv.y * 28.0);
    float t_band = floor(u_time * 24.0);
    float band_rnd = hash(vec2(band, t_band));
    float band_shift = (hash(vec2(band, t_band + 7.3)) - 0.5) * 0.06;
    sample_uv.x += band_shift * u_glitch * step(0.72, band_rnd);

    // Общее дрожание кадра
    vec2 frame_jitter = vec2(hash(vec2(t_band, 1.0)), hash(vec2(t_band, 2.0))) - 0.5;
    sample_uv += frame_jitter * 0.006 * u_glitch;

    // --- ХРОМАТИЧЕСКАЯ АБЕРРАЦИЯ ---
    // Лёгкая от интенсивности + всплеск от глитча
    vec2 dir = sample_uv - vec2(0.5);
    float dist = length(dir);
    float aberration = dist * dist * (0.06 * u_intensity + 0.35 * u_glitch);
    vec2 offset = dir * aberration;

    float r = texture2D(Texture, sample_uv + offset).r;
    float g = texture2D(Texture, sample_uv).g;
    float b = texture2D(Texture, sample_uv - offset).b;
    float a = texture2D(Texture, sample_uv).a;

    vec3 col = vec3(r, g, b);

    // --- ХОЛОДНЫЙ ГРЕЙДИНГ ---
    // Десатурация + увод в синий + лёгкое затемнение
    float luminance = dot(col, vec3(0.2126, 0.7152, 0.0722));
    col = mix(col, vec3(luminance), 0.45 * u_intensity);
    col *= mix(vec3(1.0), vec3(0.78, 0.9, 1.12), u_intensity);
    col *= mix(1.0, 0.92, u_intensity);

    // --- ГЛИТЧ: подсветка и сканлайны ---
    col.b += 0.06 * u_glitch;
    col *= 1.0 + 0.15 * u_glitch;
    float scan = 1.0 - 0.15 * step(0.5, fract(uv.y * 160.0)) * u_glitch;
    col *= scan;

    // --- ВИНЬЕТКА ---
    float vig = 1.0 - dot(dir, dir) * 1.4 * u_intensity;
    col *= clamp(vig, 0.0, 1.0);

    col = clamp(col, 0.0, 1.0);

    gl_FragColor = vec4(col, a) * color;
}
