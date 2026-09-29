//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;

struct DroidGame {
    status: Status,
    score: i64,
    jump_buffer: f32,
    coyote: f32,
    wander_spider: f32,
    ghost_fire: f32,
    no_gem_left: bool,
    start: Vec3,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            score: 0,
            jump_buffer: 0.0,
            coyote: 0.0,
            wander_spider: 0.0,
            ghost_fire: 2.0,
            no_gem_left: false,
            start: Vec3::ZERO,
        }
    }
}

impl DroidGame {
    fn lose(&mut self, cx: &mut Ctx, banner: &str) {
        if self.status != Status::Playing {
            return;
        }
        self.status = Status::Lost;
        cx.play_sound("lose");
        cx.text_at("banner", banner, 0.3, 0.4, 44.0);
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        self.start = cx.pos("player");
        // "At the start, show "GEMS {score} / 10" at the top left and play the music "E4 G4 B4 E5" at 110 beats per minute as a triangle wave." [start/init]
        // show "GEMS {score} / 10" at the top left [show/every_frame]
        cx.text_at("text_0", &format!("GEMS {} / 10", self.score), 0.02, 0.03, 24.0);
        // play the music "E4 G4 B4 E5" at 110 beats per minute as a triangle wave [music/play]
        cx.play_music(110.0, "E4 G4 B4 E5", "triangle");
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses enter, restart the game." [key_press/pressed]
        if cx.key_pressed("enter") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, show "GEMS {score} / 10" at the top left and play the music "E4 G4 B4 E5" at 110 beats per minute as a triangle wave." [show/every_frame]
        cx.text_at("text_0", &format!("GEMS {} / 10", self.score), 0.02, 0.03, 24.0);
        if self.status != Status::Playing {
            return;
        }
        let player = cx.pos("player");
        // "Move the player across the ground with the arrow keys at 7 units per second and face where it moves, smoothly." [move_player/normalized]
        let want = vec2(cx.axis_x(), cx.axis_y()).normalize_or_zero() * 7.0;
        let ease = 1.0 - (-12.0 * dt).exp();
        let v = cx.vel("player");
        cx.set_vel_x("player", v.x + (want.x - v.x) * ease);
        cx.set_vel_z("player", v.z + (-want.y - v.z) * ease);
        if want.x != 0.0 {
            cx.set_flip_x("player", want.x < 0.0);
        }
        // "The player jumps with space at 9 units per second with the jump sound." [jump/buffered]
        if cx.grounded("player") {
            self.coyote = 0.1;
        } else {
            self.coyote -= dt;
        }
        if cx.key_pressed("space") {
            self.jump_buffer = 0.12;
        } else {
            self.jump_buffer -= dt;
        }
        if self.jump_buffer > 0.0 && self.coyote > 0.0 {
            self.jump_buffer = 0.0;
            self.coyote = 0.0;
            cx.set_vel_y("player", 9.0);
            cx.play_sound("jump");
        }
        // "Each skeleton moves toward the player across the ground at 2.5 units per second within 10 units." [chase/hoisted]
        for n in cx.find_tag("skeleton") {
            let to = player - cx.pos(&n);
            if to.length() > 10.0 {
                cx.set_vel_x(&n, 0.0);
                cx.set_vel_z(&n, 0.0);
                continue;
            }
            let dir = vec3(to.x, 0.0, to.z).normalize_or_zero();
            cx.set_vel_x(&n, dir.x * 2.5);
            cx.set_vel_z(&n, dir.z * 2.5);
        }
        // "Each spider wanders across the ground at 3 units per second, turning every 1.2 seconds." [wander/shared_timer]
        self.wander_spider -= dt;
        if self.wander_spider <= 0.0 {
            self.wander_spider += 1.2;
            for n in cx.find_tag("spider") {
                let a = cx.random(0.0, std::f32::consts::TAU);
                cx.set_vel_x(&n, a.cos() * 3.0);
                cx.set_vel_z(&n, a.sin() * 3.0);
            }
        }
        // "Each ghost shoots at the player at 6 units per second every 2 seconds within 12 units, using "bomb"." [enemy_shoot/hoisted]
        self.ghost_fire -= dt;
        if self.ghost_fire <= 0.0 {
            self.ghost_fire += 2.0;
            for n in cx.find_tag("ghost") {
                let from = cx.pos(&n);
                let to = player - from;
                if to.length() > 12.0 {
                    continue;
                }
                let dir = to.normalize_or_zero();
                let shot = cx.spawn_sprite("bomb", &["bomb"], "bomb", from + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
                cx.set_solid(&shot, false);
                cx.set_vel(&shot, dir * 6.0);
            }
        }
        // "When no gems are left, the player wins." [none_left/edge]
        let none = cx.count_tag("gem") == 0;
        if none && !self.no_gem_left {
            // the player wins [win/inline]
            if self.status == Status::Playing {
                self.status = Status::Won;
                cx.play_sound("win");
                cx.text_at("banner", "YOU WIN! - PRESS ENTER", 0.3, 0.4, 44.0);
            }
        }
        self.no_gem_left = none;
        // "When the player falls below -10, the player goes back to the start." [falls_below/check]
        if cx.pos("player").y < -10.0 {
            // the player goes back to the start [back_to_start/reset]
            cx.set_pos("player", self.start);
            cx.set_vel("player", Vec3::ZERO);
        }
        // projectiles far from the player are gone (bomb)
        for n in cx.find_tag("bomb") {
            if (cx.pos(&n) - cx.pos("player")).length() > 40.0 {
                cx.destroy(&n);
            }
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When the player touches a gem, destroy it, add 1 to the score, play the pickup sound and play the spin animation on the player." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "gem") {
            // destroy it [destroy/despawn]
            cx.destroy(b);
            // add 1 to the score [score/add]
            self.score += 1;
            // play the pickup sound [sound/effect]
            cx.play_sound("pickup");
            // play the spin animation on the player [play_anim/oneshot]
            cx.play_anim("player", "spin");
        }
        // "When a bomb hits the player, destroy the bomb and the player loses." [hit/has_tag]
        if cx.has_tag(a, "bomb") && b == "player" {
            // destroy the bomb [destroy/despawn]
            cx.destroy(a);
            // the player loses [lose/helper]
            self.lose(cx, "GAME OVER - PRESS ENTER");
        }
        // "When the player touches a skeleton, the player loses "CAUGHT"." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "skeleton") {
            // the player loses "CAUGHT" [lose/helper]
            self.lose(cx, "CAUGHT - PRESS ENTER");
        }
    }

    fn status(&self) -> Status {
        self.status
    }
}

fn main() {
    droid::run::<DroidGame>(concat!(env!("CARGO_MANIFEST_DIR"), "/game"));
}
