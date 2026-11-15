use macroquad::prelude::*;

// Лужи крови под трупами: одна лужа на смерть, постепенно растекается.

// Размер "пикселя" крови в мировых единицах. Совпадает с пикселем спрайтов:
// текстура 48 px растянута до SPRITE_SIZE * SCALE = 105.6, то есть 2.2 на пиксель
const CELL: f32 = 2.2;

// Время полного растекания лужи, сек. В этих единицах время появления клеток
// упаковывается в красный канал текстуры, а возраст лужи — в цвет вершин.
const AGE_SCALE: f32 = 2.2;

const VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    // Macroquad передаёт цвет вершин сырыми байтами 0..255 (как и его внутренние шейдеры)
    color = color0 / 255.0;
}
"#;

const FRAGMENT_SHADER: &str = include_str!("../effects/blood.glsl");

// Сгусток, из которого складывается форма лужи
struct Lobe {
    offset: Vec2,
    radius: f32,
    // Задержка перед началом растекания и время растекания сгустка, сек
    delay: f32,
    spread: f32,
}

// Одна лужа: заранее испечённая текстура формы и один квад
pub struct BloodPool {
    age: f32,
    texture: Texture2D,
    // Мировой левый верхний угол и сторона квада
    quad_min: Vec2,
    quad_size: f32,
}

impl BloodPool {
    pub fn new(pos: Vec2) -> Self {
        let scale = rand::gen_range(0.85, 1.2);

        // Основной сгусток под телом появляется сразу, боковые — с задержкой
        let mut lobes = vec![Lobe {
            offset: Vec2::ZERO,
            radius: rand::gen_range(17.0, 23.0) * scale,
            delay: 0.0,
            spread: 0.9,
        }];

        let count = rand::gen_range(5usize, 9usize);
        for _ in 0..count {
            let angle = rand::gen_range(0.0, std::f32::consts::TAU);
            let dist = rand::gen_range(9.0, 28.0) * scale;
            lobes.push(Lobe {
                offset: vec2(angle.cos(), angle.sin()) * dist,
                radius: rand::gen_range(7.0, 15.0) * scale,
                delay: rand::gen_range(0.0, 0.7),
                spread: rand::gen_range(0.6, 1.2),
            });
        }

        let max_extent = lobes
            .iter()
            .map(|lobe| lobe.offset.length() + lobe.radius)
            .fold(0.0, f32::max);
        let radius_cells = (max_extent / CELL).ceil() as i32;
        let size = (radius_cells * 2 + 1) as usize;

        // Для каждой клетки запоминаем, когда до неё "дотечёт" ближайший сгусток
        let mut appear_grid = vec![f32::INFINITY; size * size];
        for lobe in &lobes {
            let cx = lobe.offset.x / CELL;
            let cy = lobe.offset.y / CELL;
            let r = lobe.radius / CELL;

            let min_ix = (cx - r).floor() as i32;
            let max_ix = (cx + r).ceil() as i32;
            let min_iy = (cy - r).floor() as i32;
            let max_iy = (cy + r).ceil() as i32;

            for iy in min_iy..=max_iy {
                for ix in min_ix..=max_ix {
                    let dx = ix as f32 - cx;
                    let dy = iy as f32 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > r {
                        continue;
                    }

                    // Края сгустка появляются позже центра, плюс немного шума
                    let appear = lobe.delay + (dist / r) * lobe.spread + rand::gen_range(-0.1, 0.1);
                    let index = (iy + radius_cells) as usize * size + (ix + radius_cells) as usize;
                    appear_grid[index] = appear_grid[index].min(appear.max(0.0));
                }
            }
        }

        // Время появления клетки — в красный канал, маска формы — в альфу.
        // Один раз при спавне, дальше лужа рисуется одним квадом без CPU-работы.
        let mut image =
            Image::gen_image_color(size as u16, size as u16, Color::new(0.0, 0.0, 0.0, 0.0));
        for iy in -radius_cells..=radius_cells {
            for ix in -radius_cells..=radius_cells {
                let index = (iy + radius_cells) as usize * size + (ix + radius_cells) as usize;
                let appear = appear_grid[index];
                if appear.is_finite() {
                    image.set_pixel(
                        (ix + radius_cells) as u32,
                        (iy + radius_cells) as u32,
                        Color::new(appear / AGE_SCALE, 0.0, 0.0, 1.0),
                    );
                }
            }
        }

        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);

        let quad_size = size as f32 * CELL;

        Self {
            age: 0.0,
            texture,
            quad_min: pos - vec2(quad_size * 0.5, quad_size * 0.5),
            quad_size,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;
    }

    pub fn draw(&self) {
        // Возраст передаём через красный канал цвета вершин, чтобы не менять
        // uniform материала посреди батча.
        let age = (self.age / AGE_SCALE).min(1.0);
        draw_texture_ex(
            &self.texture,
            self.quad_min.x,
            self.quad_min.y,
            Color::new(age, 0.0, 0.0, 1.0),
            DrawTextureParams {
                dest_size: Some(vec2(self.quad_size, self.quad_size)),
                ..Default::default()
            },
        );
    }
}

// Все лужи текущей локации
pub struct BloodManager {
    pools: Vec<BloodPool>,
    material: Material,
}

impl BloodManager {
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            MaterialParams::default(),
        )
        .unwrap();

        Self {
            pools: Vec::new(),
            material,
        }
    }

    pub fn spawn(&mut self, pos: Vec2) {
        self.pools.push(BloodPool::new(pos));
    }

    pub fn update(&mut self, dt: f32) {
        for pool in &mut self.pools {
            pool.update(dt);
        }
    }

    pub fn draw(&self) {
        if self.pools.is_empty() {
            return;
        }

        gl_use_material(&self.material);
        for pool in &self.pools {
            pool.draw();
        }
        gl_use_default_material();
    }

    pub fn clear(&mut self) {
        self.pools.clear();
    }
}
