use macroquad::prelude::*;

// Лужи крови под трупами: одна лужа на смерть, постепенно растекается.

// Размер "пикселя" крови в мировых единицах. Совпадает с пикселем спрайтов:
// текстура 48 px растянута до SPRITE_SIZE * SCALE = 105.6, то есть 2.2 на пиксель
const CELL: f32 = 2.2;

const BLOOD_COLOR: Color = Color::new(1.0, 0.0, 0.0, 1.0);

// Сгусток, из которого складывается форма лужи
struct Lobe {
    offset: Vec2,
    radius: f32,
    // Задержка перед началом растекания и время растекания сгустка, сек
    delay: f32,
    spread: f32,
}

// Клетка лужи: квадратик, который появляется в момент appear
struct Cell {
    offset: Vec2,
    appear: f32,
}

// Одна лужа
pub struct BloodPool {
    pos: Vec2,
    age: f32,
    // Отсортированы по времени появления, поэтому видимые — это cells[..visible]
    cells: Vec<Cell>,
    visible: usize,
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

        let half = CELL * 0.5;
        let mut cells = Vec::new();
        for iy in -radius_cells..=radius_cells {
            for ix in -radius_cells..=radius_cells {
                let index = (iy + radius_cells) as usize * size + (ix + radius_cells) as usize;
                let appear = appear_grid[index];
                if appear.is_finite() {
                    cells.push(Cell {
                        offset: vec2(ix as f32 * CELL - half, iy as f32 * CELL - half),
                        appear,
                    });
                }
            }
        }
        cells.sort_by(|a, b| a.appear.total_cmp(&b.appear));

        Self {
            pos,
            age: 0.0,
            cells,
            visible: 0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        while self.visible < self.cells.len() && self.cells[self.visible].appear <= self.age {
            self.visible += 1;
        }
    }

    pub fn draw(&self) {
        for cell in &self.cells[..self.visible] {
            let corner = self.pos + cell.offset;
            draw_rectangle(corner.x, corner.y, CELL, CELL, BLOOD_COLOR);
        }
    }
}

// Все лужи текущей локации
pub struct BloodManager {
    pools: Vec<BloodPool>,
}

impl BloodManager {
    pub fn new() -> Self {
        Self { pools: Vec::new() }
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
        for pool in &self.pools {
            pool.draw();
        }
    }

    pub fn clear(&mut self) {
        self.pools.clear();
    }
}
