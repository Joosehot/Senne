//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;

struct DroidGame {
    status: Status,
    lives: i64,
    score: i64,
    wave: i64,
    shot_timer: f32,
    march_alien: f32,
    most_alien: f32,
    timer: f32,
    safe: f32,
    no_alien_left: bool,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            lives: 3,
            score: 0,
            wave: 1,
            shot_timer: 0.0,
            march_alien: 1.0,
            most_alien: 0.0,
            timer: 1.2,
            safe: 0.0,
            no_alien_left: false,
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
        // "At the start, the player has 3 lives, show "SCORE {score}" at the top left, show "WAVE {wave}" at the top right, show "LIVES {lives}" at the top and play the music "C3 C3 G2 G2" at 90 beats per minute as a saw wave." [start/init]
        // the player has 3 lives [lives/set]
        self.lives = 3;
        // show "SCORE {score}" at the top left [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {}", self.score), 0.02, 0.03, 24.0);
        // show "WAVE {wave}" at the top right [show/every_frame]
        cx.text_at("text_1", &format!("WAVE {}", self.wave), 0.78, 0.03, 24.0);
        // show "LIVES {lives}" at the top [show/every_frame]
        cx.text_at("text_2", &format!("LIVES {}", self.lives), 0.42, 0.03, 24.0);
        // play the music "C3 C3 G2 G2" at 90 beats per minute as a saw wave [music/play]
        cx.play_music(90.0, "C3 C3 G2 G2", "saw");
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, the player has 3 lives, show "SCORE {score}" at the top left, show "WAVE {wave}" at the top right, show "LIVES {lives}" at the top and play the music "C3 C3 G2 G2" at 90 beats per minute as a saw wave." [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {}", self.score), 0.02, 0.03, 24.0);
        // "At the start, the player has 3 lives, show "SCORE {score}" at the top left, show "WAVE {wave}" at the top right, show "LIVES {lives}" at the top and play the music "C3 C3 G2 G2" at 90 beats per minute as a saw wave." [show/every_frame]
        cx.text_at("text_1", &format!("WAVE {}", self.wave), 0.78, 0.03, 24.0);
        // "At the start, the player has 3 lives, show "SCORE {score}" at the top left, show "WAVE {wave}" at the top right, show "LIVES {lives}" at the top and play the music "C3 C3 G2 G2" at 90 beats per minute as a saw wave." [show/every_frame]
        cx.text_at("text_2", &format!("LIVES {}", self.lives), 0.42, 0.03, 24.0);
        if self.status != Status::Playing {
            return;
        }
        // blink while safe after losing a life
        if self.safe > 0.0 {
            self.safe -= dt;
            let lit = self.safe <= 0.0 || (self.safe * 12.0) as i64 % 2 == 0;
            cx.set_color("player", if lit { Vec3::ONE } else { Vec3::splat(0.35) });
        }
        // "Move the player sideways with the arrow keys at 9 units per second within 11 units." [move_player/axis]
        let want = cx.axis_x() * 9.0;
        cx.set_vel_x("player", want);
        let p = cx.pos("player");
        if p.x.abs() > 11.0 {
            cx.set_pos("player", vec3(p.x.clamp(-11.0, 11.0), p.y, p.z));
            cx.set_vel_x("player", 0.0);
        }
        // "The player shoots up with space at 22 units per second every 0.3 seconds, using "laser", with the laser sound." [player_shoot/countdown]
        self.shot_timer -= dt;
        if cx.key("space") && self.shot_timer <= 0.0 {
            self.shot_timer = 0.3;
            let dir = vec2(0.0, 1.0);
            let shot = cx.spawn_sprite("shot", &["shot"], "laser", cx.pos("player") + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
            cx.set_solid(&shot, false);
            cx.set_vel(&shot, dir * 22.0);
            cx.play_sound("laser");
        }
        // "Each alien marches sideways at 1.4 units per second within 12.5 units, stepping down 0.6 units, up to 2.5 times as fewer are left, 15 percent faster each wave." [march/move_by]
        let group = cx.find_tag("alien");
        if !group.is_empty() {
            let left = group.len() as f32;
            self.most_alien = self.most_alien.max(left);
            let speed = 1.4 * (1.0 + 1.5 * (1.0 - left / self.most_alien)) * (1.0 + 0.15 * (self.wave - 1) as f32);
            let step = self.march_alien * speed * dt;
            let xs: Vec<f32> = group.iter().map(|n| cx.pos(n).x).collect();
            let lo = xs.iter().copied().fold(f32::INFINITY, f32::min);
            let hi = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            if (step > 0.0 && hi + step > 12.5) || (step < 0.0 && lo + step < -12.5) {
                self.march_alien = -self.march_alien;
                for n in &group {
                    cx.move_by(n, vec2(0.0, -0.6));
                }
            } else {
                for n in &group {
                    cx.move_by(n, vec2(step, 0.0));
                }
            }
        }
        // "Animate each alien with "alien_1" and "alien_2" at 2 frames per second." [animate/clock]
        let frame = ["alien_1", "alien_2"][(cx.time() * 2.0) as usize % 2];
        for n in cx.find_tag("alien") {
            cx.set_sprite(&n, frame);
        }
        // "Every 1.2 seconds, a random alien shoots down at 7 units per second, using "bomb"." [every/countdown]
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.timer += 1.2;
            // a random alien shoots down at 7 units per second, using "bomb" [random_shot/pick]
            let group = cx.find_tag("alien");
            if !group.is_empty() {
                let pick = &group[(cx.random(0.0, group.len() as f32) as usize).min(group.len() - 1)];
                let from = cx.pos(pick);
                let dir = vec2(0.0, -1.0);
                let shot = cx.spawn_sprite("bomb", &["bomb"], "bomb", from + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
                cx.set_solid(&shot, false);
                cx.set_vel(&shot, dir * 7.0);
            }
        }
        // "When no aliens are left, destroy the bombs and start the next wave of aliens." [none_left/edge]
        let none = cx.count_tag("alien") == 0;
        if none && !self.no_alien_left {
            // destroy the bombs [destroy/despawn]
            cx.destroy_tag("bomb");
            // start the next wave of aliens [next_wave/respawn]
            self.wave += 1;
            cx.respawn_tag("alien");
        }
        self.no_alien_left = none;
        // "When an alien falls below -6, the player loses." [falls_below/check]
        for n in cx.find_tag("alien") {
            if cx.pos(&n).y < -6.0 {
                // the player loses [lose/helper]
                self.lose(cx, "GAME OVER - PRESS R");
            }
        }
        // projectiles far from the player are gone (bomb)
        for n in cx.find_tag("bomb") {
            if (cx.pos(&n) - cx.pos("player")).length() > 40.0 {
                cx.destroy(&n);
            }
        }
        // projectiles far from the player are gone (shot)
        for n in cx.find_tag("shot") {
            if (cx.pos(&n) - cx.pos("player")).length() > 40.0 {
                cx.destroy(&n);
            }
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When a shot hits an alien, destroy both, add 10 points and play the explosion sound." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "alien") {
            // destroy both [destroy/despawn]
            cx.destroy(a);
            cx.destroy(b);
            // add 10 points [score/add]
            self.score += 10;
            // play the explosion sound [sound/effect]
            cx.play_sound("explosion");
        }
        // "When a shot hits a shield, destroy the shot." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "shield") {
            // destroy the shot [destroy/despawn]
            cx.destroy(a);
        }
        // "When a bomb hits a shield, destroy both." [hit/has_tag]
        if cx.has_tag(a, "bomb") && cx.has_tag(b, "shield") {
            // destroy both [destroy/despawn]
            cx.destroy(a);
            cx.destroy(b);
        }
        // "When a bomb hits the player, destroy the bomb, the player loses a life, safe for 2 seconds." [hit/has_tag]
        if cx.has_tag(a, "bomb") && b == "player" {
            // destroy the bomb [destroy/despawn]
            cx.destroy(a);
            // the player loses a life, safe for 2 seconds [hurt/inline]
            if self.safe <= 0.0 {
                self.lives -= 1;
                cx.play_sound("hurt");
                if self.lives <= 0 {
                    self.lose(cx, "GAME OVER - PRESS R");
                } else {
                    self.safe = 2.0;
                }
            }
        }
    }

    fn status(&self) -> Status {
        self.status
    }
}

fn main() {
    droid::run::<DroidGame>(concat!(env!("CARGO_MANIFEST_DIR"), "/game"));
}
