use crate::assets::Assets;
use crate::entity::{AnimationState, draw_char_tinted};
use crate::player::Player;
use macroquad::prelude::*;

const RIPPLE_CONFIG: [(f32, f32); 3] = [(340.0, 0.45), (520.0, 0.65), (720.0, 0.9)];

const RIPPLE_COLOR: Color = Color::new(0.68, 0.88, 1.0, 1.0);

const GHOST_COLOR: Color = Color::new(0.65, 0.85, 1.0, 1.0);
const GHOST_ALPHA: f32 = 0.35;
const GHOST_LIFE: f32 = 0.65;
const GHOST_SPAWN_INTERVAL: f32 = 0.04;
const MAX_GHOSTS: usize = 30;

struct Ripple {
    pos: Vec2,
    age: f32,
    duration: f32,
    max_radius: f32,
}

impl Ripple {
    fn update(&mut self, dt: f32) -> bool {
        self.age += dt;
        self.age < self.duration
    }

    fn draw(&self) {
        let t = (self.age / self.duration).clamp(0.0, 1.0);
        let radius = self.max_radius * (1.0 - (1.0 - t).powi(3));
        let alpha = (1.0 - t) * 0.5;

        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            radius,
            4.0,
            Color::new(RIPPLE_COLOR.r, RIPPLE_COLOR.g, RIPPLE_COLOR.b, alpha),
        );
    }
}

struct Ghost {
    pos: Vec2,
    rotation: f32,
    legs_rotation: f32,
    torso_frame: usize,
    torso_row: f32,
    legs_frame: usize,
    legs_row: f32,
    is_dead: bool,
    age: f32,
}

impl Ghost {
    fn draw(&self, assets: &Assets) {
        let alpha = (1.0 - self.age / GHOST_LIFE).clamp(0.0, 1.0) * GHOST_ALPHA;
        let color = Color::new(GHOST_COLOR.r, GHOST_COLOR.g, GHOST_COLOR.b, alpha);

        let mut torso = AnimationState::new(1, 1.0, 0);
        torso.current_frame = self.torso_frame;
        torso.row_index = self.torso_row;

        let mut legs = AnimationState::new(1, 1.0, 0);
        legs.current_frame = self.legs_frame;
        legs.row_index = self.legs_row;

        draw_char_tinted(
            &assets.player,
            self.pos,
            self.rotation,
            self.legs_rotation,
            &torso,
            &legs,
            self.is_dead,
            false,
            color,
        );
    }
}

pub struct TimeSlowFx {
    ripples: Vec<Ripple>,
    ghosts: Vec<Ghost>,
    ghost_timer: f32,
    was_active: bool,
}

impl TimeSlowFx {
    pub fn new() -> Self {
        Self {
            ripples: Vec::new(),
            ghosts: Vec::new(),
            ghost_timer: 0.0,
            was_active: false,
        }
    }

    pub fn clear(&mut self) {
        self.ripples.clear();
        self.ghosts.clear();
        self.ghost_timer = 0.0;
        self.was_active = false;
    }

    pub fn update(&mut self, dt: f32, player: &Player, intensity: f32, active: bool) {
        if active && !self.was_active {
            for (max_radius, duration) in RIPPLE_CONFIG {
                self.ripples.push(Ripple {
                    pos: player.pos,
                    age: 0.0,
                    duration,
                    max_radius,
                });
            }
        }
        self.was_active = active;

        self.ripples.retain_mut(|ripple| ripple.update(dt));

        if active && intensity > 0.2 && !player.is_dead {
            self.ghost_timer -= dt;
            if self.ghost_timer <= 0.0 {
                self.spawn_ghost(player);
                self.ghost_timer = GHOST_SPAWN_INTERVAL;
            }
        } else {
            self.ghost_timer = 0.0;
        }

        for ghost in &mut self.ghosts {
            ghost.age += dt;
        }
        self.ghosts.retain(|ghost| ghost.age < GHOST_LIFE);
    }

    fn spawn_ghost(&mut self, player: &Player) {
        if self.ghosts.len() >= MAX_GHOSTS {
            self.ghosts.remove(0);
        }

        self.ghosts.push(Ghost {
            pos: player.pos,
            rotation: player.rotation,
            legs_rotation: player.legs_rotation,
            torso_frame: player.torso_anim.current_frame,
            torso_row: player.torso_anim.row_index,
            legs_frame: player.legs_anim.current_frame,
            legs_row: player.legs_anim.row_index,
            is_dead: player.is_dead,
            age: 0.0,
        });
    }

    pub fn draw_rings(&self) {
        for ripple in &self.ripples {
            ripple.draw();
        }
    }

    pub fn draw_ghosts(&self, assets: &Assets) {
        for ghost in &self.ghosts {
            ghost.draw(assets);
        }
    }
}
