use crate::objects::Bullet;
use macroquad::prelude::*;
use miniquad::{BlendFactor, BlendState, BlendValue, Equation};

const PLAYER_LIGHT_RADIUS: f32 = 240.0;
const PLAYER_LIGHT_INTENSITY: f32 = 0.9;
const PLAYER_LIGHT_COLOR: Color = Color::new(1.0, 0.92, 0.78, 1.0);

const BULLET_LIGHT_RADIUS: f32 = 50.0;
const BULLET_LIGHT_INTENSITY: f32 = 0.5;
const BULLET_LIGHT_COLOR: Color = Color::new(1.0, 0.85, 0.4, 1.0);

// Холодная палитра освещения на время замедления
const SLOW_AMBIENT_COLOR: Color = Color::new(0.09, 0.12, 0.24, 1.0);
const SLOW_PLAYER_LIGHT_COLOR: Color = Color::new(0.62, 0.85, 1.0, 1.0);

pub const MAP_STATIC_LIGHT_COLOR: Color = Color::new(1.0, 0.92, 0.78, 1.0);
pub const MAP_STATIC_LIGHT_RADIUS: f32 = 500.0;
pub const MAP_STATIC_LIGHT_INTENSITY: f32 = 1.0;
pub const MAP_STATIC_LIGHT_FLICKER: f32 = 0.0;

// Запечённая текстура статического света не должна превышать лимит GPU.
// Для текущих карт масштаб остаётся 1:1 (одна мировая единица на тексель).
const MAX_BAKE_SIZE: f32 = 4096.0;

#[derive(Clone, Copy)]
pub struct Light {
    pub pos: Vec2,
    pub radius: f32,
    pub color: Color,
    pub intensity: f32,
    pub flicker: f32,
}

pub const fn lamp(pos: Vec2, radius: f32, color: Color, intensity: f32, flicker: f32) -> Light {
    Light {
        pos,
        radius,
        color,
        intensity,
        flicker,
    }
}

const LIGHT_VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying mediump vec2 uv;
varying mediump vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}
"#;

// Цвет и интенсивность источника приходят через цвет вершин. Так все
// динамические источники рисуются одним материалом и уходят в один батч.
const LIGHT_FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;

varying mediump vec2 uv;
varying mediump vec4 color;

void main() {
    float dist = length(uv - vec2(0.5)) * 2.0;
    float t = max(1.0 - dist, 0.0);
    float falloff = t * t * (3.0 - 2.0 * t);
    gl_FragColor = vec4(color.rgb * falloff, 1.0);
}
"#;

const COMPOSITE_FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;

varying mediump vec2 uv;
varying mediump vec4 color;

uniform sampler2D Texture;

void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}
"#;

struct BakedStaticLight {
    // RenderTarget хранится целиком: его удаление освобождает и color texture.
    target: RenderTarget,
    // Мировые границы, которые покрывает запечённая текстура.
    bounds: Rect,
}

pub struct LightingManager {
    pub ambient_color: Color,
    pub enabled: bool,
    light_map: RenderTarget,
    light_material: Material,
    static_material: Material,
    composite_material: Material,
    baked_static_light: Option<BakedStaticLight>,
    flickering_lights: Vec<Light>,
}

impl LightingManager {
    pub fn new() -> Self {
        let light_material = load_material(
            ShaderSource::Glsl {
                vertex: LIGHT_VERTEX_SHADER,
                fragment: LIGHT_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: additive_pipeline(),
                ..Default::default()
            },
        )
        .unwrap();

        let static_material = load_material(
            ShaderSource::Glsl {
                vertex: LIGHT_VERTEX_SHADER,
                fragment: COMPOSITE_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: additive_pipeline(),
                ..Default::default()
            },
        )
        .unwrap();

        let composite_material = load_material(
            ShaderSource::Glsl {
                vertex: LIGHT_VERTEX_SHADER,
                fragment: COMPOSITE_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::DestinationColor),
                        BlendFactor::Zero,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        let width = screen_width() as u32;
        let height = screen_height() as u32;

        Self {
            ambient_color: Color::new(0.16, 0.17, 0.24, 1.0),
            enabled: true,
            light_map: render_target(width, height),
            light_material,
            static_material,
            composite_material,
            baked_static_light: None,
            flickering_lights: Vec::new(),
        }
    }

    /// Запекает неподвижные источники света в мировую текстуру.
    /// Вызывается при загрузке карты и при каждой смене локации.
    pub fn bake_static_lights(&mut self, bounds: Rect, lights: &[Light]) {
        self.flickering_lights.clear();

        let mut baked = Vec::with_capacity(lights.len());
        for &light in lights {
            if light.flicker > 0.0 {
                // Мерцающие источники нельзя запекать: они меняются каждый кадр.
                self.flickering_lights.push(light);
            } else {
                baked.push(light);
            }
        }

        if baked.is_empty() || bounds.w <= 0.0 || bounds.h <= 0.0 {
            self.baked_static_light = None;
            return;
        }

        let bake_scale = (MAX_BAKE_SIZE / bounds.w.max(bounds.h)).min(1.0);
        let width = (bounds.w * bake_scale).ceil().max(1.0) as u32;
        let height = (bounds.h * bake_scale).ceil().max(1.0) as u32;

        let target = render_target(width, height);
        target.texture.set_filter(FilterMode::Linear);

        let mut camera = Camera2D::from_display_rect(bounds);
        camera.render_target = Some(target.clone());

        set_camera(&camera);
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));

        gl_use_material(&self.light_material);
        for light in baked {
            draw_light_quad(light.pos, light.radius, light_vertex_color(light, 0.0));
        }
        gl_use_default_material();

        set_default_camera();

        self.baked_static_light = Some(BakedStaticLight { target, bounds });
    }

    pub fn draw(
        &mut self,
        camera: &Camera2D,
        player_pos: Vec2,
        bullets: &[Bullet],
        slow_intensity: f32,
    ) {
        if !self.enabled {
            return;
        }

        self.sync_light_map_size();

        let view = ViewTransform::from_camera(camera);
        let time = get_time() as f32;

        set_camera(&self.light_map_camera());
        clear_background(lerp_color(
            self.ambient_color,
            SLOW_AMBIENT_COLOR,
            slow_intensity,
        ));

        // Статический свет уже посчитан в мировой текстуре — один квад.
        if let Some(baked) = &self.baked_static_light {
            let top_left = view.to_screen(vec2(baked.bounds.x, baked.bounds.y));
            let scale = view.world_to_screen_scale();

            gl_use_material(&self.static_material);
            draw_texture_ex(
                &baked.target.texture,
                top_left.x,
                top_left.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(baked.bounds.w * scale, baked.bounds.h * scale)),
                    flip_y: true,
                    ..Default::default()
                },
            );
            gl_use_default_material();
        }

        // Динамический свет: один материал и один батч на все источники.
        gl_use_material(&self.light_material);
        for light in self.flickering_lights.iter().copied() {
            draw_light(&view, light, time);
        }
        draw_light(&view, player_light(player_pos, slow_intensity), time);
        for bullet in bullets {
            draw_light(&view, bullet_light(bullet, slow_intensity), time);
        }
        gl_use_default_material();

        set_default_camera();
        gl_use_material(&self.composite_material);
        draw_texture_ex(
            &self.light_map.texture,
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
    }

    fn sync_light_map_size(&mut self) {
        let width = screen_width() as u32;
        let height = screen_height() as u32;

        if self.light_map.texture.width() as u32 != width
            || self.light_map.texture.height() as u32 != height
        {
            self.light_map = render_target(width, height);
        }
    }

    fn light_map_camera(&self) -> Camera2D {
        let mut camera =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        camera.render_target = Some(self.light_map.clone());
        camera
    }
}

fn additive_pipeline() -> PipelineParams {
    PipelineParams {
        color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::One,
            BlendFactor::One,
        )),
        ..Default::default()
    }
}

fn draw_light(view: &ViewTransform, light: Light, time: f32) {
    let center = view.to_screen(light.pos);
    let radius = view.radius_to_screen(light.radius);
    if !view.is_visible(center, radius) {
        return;
    }

    draw_light_quad(center, radius, light_vertex_color(light, time));
}

fn draw_light_quad(center: Vec2, radius: f32, color: Color) {
    draw_rectangle(
        center.x - radius,
        center.y - radius,
        radius * 2.0,
        radius * 2.0,
        color,
    );
}

fn light_vertex_color(light: Light, time: f32) -> Color {
    let intensity = light.intensity * flicker_multiplier(light, time);
    Color::new(
        (light.color.r * intensity).clamp(0.0, 1.0),
        (light.color.g * intensity).clamp(0.0, 1.0),
        (light.color.b * intensity).clamp(0.0, 1.0),
        1.0,
    )
}

fn player_light(pos: Vec2, slow_intensity: f32) -> Light {
    lamp(
        pos,
        PLAYER_LIGHT_RADIUS * (1.0 + 0.15 * slow_intensity),
        lerp_color(PLAYER_LIGHT_COLOR, SLOW_PLAYER_LIGHT_COLOR, slow_intensity),
        PLAYER_LIGHT_INTENSITY,
        0.0,
    )
}

fn bullet_light(bullet: &Bullet, slow_intensity: f32) -> Light {
    lamp(
        bullet.pos,
        BULLET_LIGHT_RADIUS * (1.0 + 0.5 * slow_intensity),
        BULLET_LIGHT_COLOR,
        BULLET_LIGHT_INTENSITY * (1.0 + 0.3 * slow_intensity),
        0.0,
    )
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        from.r + (to.r - from.r) * t,
        from.g + (to.g - from.g) * t,
        from.b + (to.b - from.b) * t,
        from.a + (to.a - from.a) * t,
    )
}

fn flicker_multiplier(light: Light, time: f32) -> f32 {
    if light.flicker <= 0.0 {
        return 1.0;
    }

    let phase = light.pos.x * 0.017 + light.pos.y * 0.011;
    let wave = (time * 7.0 + phase).sin() * 0.6 + (time * 19.0 + phase).sin() * 0.4;
    1.0 - light.flicker * (0.5 + 0.5 * wave)
}

struct ViewTransform {
    target: Vec2,
    zoom: Vec2,
    screen_size: Vec2,
}

impl ViewTransform {
    fn from_camera(camera: &Camera2D) -> Self {
        Self {
            target: camera.target,
            zoom: camera.zoom,
            screen_size: vec2(screen_width(), screen_height()),
        }
    }

    fn to_screen(&self, world_pos: Vec2) -> Vec2 {
        vec2(
            self.screen_size.x * (0.5 + 0.5 * self.zoom.x * (world_pos.x - self.target.x)),
            self.screen_size.y * (0.5 + 0.5 * self.zoom.y * (world_pos.y - self.target.y)),
        )
    }

    fn world_to_screen_scale(&self) -> f32 {
        self.zoom.x * self.screen_size.x * 0.5
    }

    fn radius_to_screen(&self, world_radius: f32) -> f32 {
        world_radius * self.world_to_screen_scale()
    }

    fn is_visible(&self, center: Vec2, radius: f32) -> bool {
        center.x + radius > 0.0
            && center.x - radius < self.screen_size.x
            && center.y + radius > 0.0
            && center.y - radius < self.screen_size.y
    }
}
