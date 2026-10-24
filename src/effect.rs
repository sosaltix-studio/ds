use macroquad::prelude::*;

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

const TRIP_FRAGMENT_SHADER: &str = include_str!("../effects/da.glsl");
const TIME_SLOW_FRAGMENT_SHADER: &str = include_str!("../effects/time_slow.glsl");

// Скорость нарастания/затухания визуальной интенсивности (в секунду)
const INTENSITY_SPEED: f32 = 5.0;

// Длительность глитч-берста на входе/выходе из замедления
const GLITCH_DURATION: f32 = 0.18;

// Эффект "трипа" (story effect)
pub struct TripEffect {
    pub active: bool,
    pub intensity: f32,
    target_intensity: f32,
    material: Material,
    pub trail_target: RenderTarget,
    pub time: f32,
}

impl TripEffect {
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: TRIP_FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("u_time", UniformType::Float1),
                    UniformDesc::new("u_intensity", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .unwrap();

        let w = screen_width() as u32;
        let h = screen_height() as u32;

        let trail_target = render_target(w, h);
        trail_target.texture.set_filter(FilterMode::Nearest);

        Self {
            active: false,
            intensity: 0.0,
            target_intensity: 0.0,
            material,
            trail_target,
            time: 0.0,
        }
    }

    pub fn toggle(&mut self, enable: bool) {
        self.active = enable;
        self.target_intensity = if enable { 1.0 } else { 0.0 };
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        let speed = 2.5;
        if self.intensity < self.target_intensity {
            self.intensity = (self.intensity + dt * speed).min(self.target_intensity);
        } else if self.intensity > self.target_intensity {
            self.intensity = (self.intensity - dt * speed).max(self.target_intensity);
        }

        let w = screen_width() as u32;
        let h = screen_height() as u32;
        if self.trail_target.texture.width() as u32 != w
            || self.trail_target.texture.height() as u32 != h
        {
            self.trail_target = render_target(w, h);
            self.trail_target.texture.set_filter(FilterMode::Nearest);
        }
    }

    // input — исходное изображение (сцена или результат другого пост-эффекта).
    // Вход всегда в "перевёрнутой" ориентации — сцена рендерится игровой камерой,
    // а пост-эффекты её сохраняют, поэтому квад рисуется с flip_y: true
    pub fn render_to_screen(&self, input: &Texture2D) {
        let mut trail_cam =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        trail_cam.render_target = Some(self.trail_target.clone());
        set_camera(&trail_cam);

        let fade_alpha = 0.5 - (0.96 * self.intensity).clamp(0.0, 1.0);

        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, fade_alpha),
        );

        draw_texture_ex(
            input,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );

        set_default_camera();
        self.material.set_uniform("u_time", self.time);
        self.material.set_uniform("u_intensity", self.intensity);

        gl_use_material(&self.material);

        draw_texture_ex(
            &self.trail_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );

        gl_use_default_material();
    }
}

// Эффект замедления времени (холодный грейдинг + виньетка + глитч-берсты)
pub struct TimeSlowEffect {
    pub intensity: f32,
    pub glitch_strength: f32,
    target_intensity: f32,
    active: bool,
    glitch_timer: f32,
    material: Material,
    out_target: RenderTarget,
    time: f32,
}

impl TimeSlowEffect {
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: TIME_SLOW_FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("u_time", UniformType::Float1),
                    UniformDesc::new("u_intensity", UniformType::Float1),
                    UniformDesc::new("u_glitch", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .unwrap();

        let w = screen_width() as u32;
        let h = screen_height() as u32;

        let out_target = render_target(w, h);
        out_target.texture.set_filter(FilterMode::Nearest);

        Self {
            intensity: 0.0,
            glitch_strength: 0.0,
            target_intensity: 0.0,
            active: false,
            glitch_timer: 0.0,
            material,
            out_target,
            time: 0.0,
        }
    }

    // Переключение способности. Глитч-берст запускается и на входе, и на выходе
    pub fn set_active(&mut self, active: bool) {
        if self.active == active {
            return;
        }

        self.active = active;
        self.target_intensity = if active { 1.0 } else { 0.0 };
        self.glitch_timer = GLITCH_DURATION;
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        if self.intensity < self.target_intensity {
            self.intensity = (self.intensity + dt * INTENSITY_SPEED).min(self.target_intensity);
        } else if self.intensity > self.target_intensity {
            self.intensity = (self.intensity - dt * INTENSITY_SPEED).max(self.target_intensity);
        }

        // Огибающая глитча: быстро разгорается и гаснет (sin по прогрессу берста)
        if self.glitch_timer > 0.0 {
            self.glitch_timer = (self.glitch_timer - dt).max(0.0);
            let progress = 1.0 - self.glitch_timer / GLITCH_DURATION;
            self.glitch_strength = (progress * std::f32::consts::PI).sin();
        } else {
            self.glitch_strength = 0.0;
        }

        let w = screen_width() as u32;
        let h = screen_height() as u32;
        if self.out_target.texture.width() as u32 != w
            || self.out_target.texture.height() as u32 != h
        {
            self.out_target = render_target(w, h);
            self.out_target.texture.set_filter(FilterMode::Nearest);
        }
    }

    // Эффект считается активным, пока виден хотя бы глитч-берст затухания
    pub fn is_active(&self) -> bool {
        self.intensity > 0.001 || self.glitch_strength > 0.001
    }

    // Пропускает изображение через шейдер замедления.
    // Вход, как и у TripEffect, всегда в "перевёрнутой" ориентации
    pub fn process(&self, input: &Texture2D) {
        let mut out_cam =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        out_cam.render_target = Some(self.out_target.clone());
        set_camera(&out_cam);

        self.material.set_uniform("u_time", self.time);
        self.material.set_uniform("u_intensity", self.intensity);
        self.material.set_uniform("u_glitch", self.glitch_strength);

        gl_use_material(&self.material);

        draw_texture_ex(
            input,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );

        gl_use_default_material();
        set_default_camera();
    }

    pub fn out_texture(&self) -> Texture2D {
        self.out_target.texture.clone()
    }
}
