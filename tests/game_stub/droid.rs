//! A signature stub of Droid's game API (`AIgame-engine/src/api.rs`, `droid::prelude`,
//! `droid::run`), so `cargo test` can type-check Senne's game goldens with
//! `rustc -D warnings` without building the engine. Bodies do nothing.
//! Keep the signatures in step with Droid; the real check builds the goldens
//! against the droid crate itself.

#![allow(dead_code, unused_variables)]

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const X: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
    pub fn splat(v: f32) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
    }
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn normalize(self) -> Vec3 {
        self * (1.0 / self.length())
    }
    pub fn normalize_or_zero(self) -> Vec3 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            Vec3::ZERO
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

pub fn vec2(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BodyKind {
    #[default]
    None,
    Static,
    Kinematic,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Won,
    Lost,
}

pub trait Game {
    fn init(&mut self, _cx: &mut Ctx) {}
    fn update(&mut self, _cx: &mut Ctx, _dt: f32) {}
    fn on_collision(&mut self, _cx: &mut Ctx, _a: &str, _b: &str) {}
    fn status(&self) -> Status {
        Status::Playing
    }
}

pub struct Ctx<'a> {
    w: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> Ctx<'a> {
    pub fn time(&self) -> f32 {
        0.0
    }
    pub fn key(&self, k: &str) -> bool {
        false
    }
    pub fn key_pressed(&self, k: &str) -> bool {
        false
    }
    pub fn axis_x(&self) -> f32 {
        0.0
    }
    pub fn axis_y(&self) -> f32 {
        0.0
    }
    pub fn voice(&self) -> f32 {
        0.0
    }
    pub fn random(&self, lo: f32, hi: f32) -> f32 {
        lo
    }
    pub fn exists(&self, n: &str) -> bool {
        false
    }
    pub fn pos(&self, n: &str) -> Vec3 {
        Vec3::ZERO
    }
    pub fn vel(&self, n: &str) -> Vec3 {
        Vec3::ZERO
    }
    pub fn grounded(&self, n: &str) -> bool {
        false
    }
    pub fn has_tag(&self, n: &str, tag: &str) -> bool {
        false
    }
    pub fn find_tag(&self, tag: &str) -> Vec<String> {
        Vec::new()
    }
    pub fn count_tag(&self, tag: &str) -> usize {
        0
    }
    pub fn set_pos(&mut self, n: &str, p: Vec3) {}
    pub fn move_by(&mut self, n: &str, d: Vec3) {}
    pub fn set_vel(&mut self, n: &str, v: Vec3) {}
    pub fn set_vel_x(&mut self, n: &str, x: f32) {}
    pub fn set_vel_y(&mut self, n: &str, y: f32) {}
    pub fn set_vel_z(&mut self, n: &str, z: f32) {}
    pub fn set_scale(&mut self, n: &str, s: Vec3) {}
    pub fn set_rotation(&mut self, n: &str, degrees: Vec3) {}
    pub fn set_color(&mut self, n: &str, c: Vec3) {}
    pub fn set_flip_x(&mut self, n: &str, f: bool) {}
    pub fn set_solid(&mut self, n: &str, s: bool) {}
    pub fn set_sprite(&mut self, n: &str, sprite: &str) {}
    pub fn add_tag(&mut self, n: &str, tag: &str) {}
    pub fn animate_sprite(&mut self, n: &str, frames: &[&str], fps: f32) {}
    pub fn squash(&mut self, n: &str, amount: f32) {}
    pub fn spawn_sprite(&mut self, name: &str, tags: &[&str], sprite: &str, pos: Vec3, scale: Vec3, body: BodyKind) -> String {
        name.to_string()
    }
    pub fn spawn_shape(&mut self, name: &str, tags: &[&str], shape: &str, pos: Vec3, scale: Vec3, color: Vec3, body: BodyKind) -> String {
        name.to_string()
    }
    pub fn play_anim(&mut self, n: &str, anim: &str) {}
    pub fn remove_text(&mut self, id: &str) {}
    pub fn destroy(&mut self, n: &str) {}
    pub fn respawn_tag(&mut self, tag: &str) -> usize {
        0
    }
    pub fn respawn(&mut self, name: &str) -> Option<String> {
        None
    }
    pub fn destroy_tag(&mut self, tag: &str) {}
    pub fn text(&mut self, id: &str, s: &str) {}
    pub fn text_at(&mut self, id: &str, s: &str, x: f32, y: f32, size: f32) {}
    pub fn text_color(&mut self, id: &str, c: Vec3) {}
    pub fn play_sound(&mut self, name: &str) {}
    pub fn play_music(&mut self, bpm: f32, notes: &str, wave: &str) {}
    pub fn restart(&mut self) {}
}

pub mod prelude {
    pub use crate::{vec2, vec3, BodyKind, Ctx, Game, Status, Vec3};
}

pub fn run<G: Game + Default + 'static>(data_dir: &str) {}
