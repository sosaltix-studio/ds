#version 100
precision mediump float;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;

// Красный канал текстуры — время появления клетки, альфа — маска формы.
// Красный канал цвета вершин — возраст лужи в тех же единицах.
void main() {
    vec4 texel = texture2D(Texture, uv);
    // Пустые пиксели и клетки, которые ещё не "дотекли", отбрасываем:
    // так не нужны блендинг и сортировка.
    if (texel.a < 0.5 || texel.r > color.r) {
        discard;
    }

    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
