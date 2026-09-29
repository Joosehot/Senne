//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;

struct DroidGame {
    status: Status,
    lives: i64,
    clock: f32,
    score: i64,
    aim: Vec3,
    shot_timer: f32,
    wander_blob: f32,
    safe: f32,
    time_60_0: bool,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            lives: 3,
            clock: 0.0,
            score: 0,
            aim: Vec3::X,
            shot_timer: 0.0,
            wander_blob: 0.0,
            safe: 0.0,
            time_60_0: false,
        }
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        // "At the start, the player has 3 lives, show "TIME {countdown 60}" at the top and show "LIVES {lives} SCORE {score}" at the top left." [start/init]
        // the player has 3 lives [lives/set]
        self.lives = 3;
        // show "TIME {countdown 60}" at the top [show/every_frame]
        cx.text_at("text_0", &format!("TIME {}", (60.0 - self.clock).max(0.0).ceil() as i64), 0.42, 0.03, 24.0);
        // show "LIVES {lives}  SCORE {score}" at the top left [show/every_frame]
        cx.text_at("text_1", &format!("LIVES {}  SCORE {}", self.lives, self.score), 0.02, 0.03, 24.0);
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, the player has 3 lives, show "TIME {countdown 60}" at the top and show "LIVES {lives} SCORE {score}" at the top left." [show/every_frame]
        cx.text_at("text_0", &format!("TIME {}", (60.0 - self.clock).max(0.0).ceil() as i64), 0.42, 0.03, 24.0);
        // "At the start, the player has 3 lives, show "TIME {countdown 60}" at the top and show "LIVES {lives} SCORE {score}" at the top left." [show/every_frame]
        cx.text_at("text_1", &format!("LIVES {}  SCORE {}", self.lives, self.score), 0.02, 0.03, 24.0);
        if self.status != Status::Playing {
            return;
        }
        self.clock += dt;
        let player = cx.pos("player");
        // blink while safe after losing a life
        if self.safe > 0.0 {
            self.safe -= dt;
            let lit = self.safe <= 0.0 || (self.safe * 12.0) as i64 % 2 == 0;
            cx.set_color("player", if lit { Vec3::ONE } else { Vec3::splat(0.35) });
        }
        // "Move the player with the arrow keys at 7 units per second and face where it moves." [move_player/normalized]
        let want = vec2(cx.axis_x(), cx.axis_y()).normalize_or_zero() * 7.0;
        cx.set_vel_x("player", want.x);
        cx.set_vel_y("player", want.y);
        if want.x != 0.0 {
            cx.set_flip_x("player", want.x < 0.0);
        }
        if want != Vec3::ZERO {
            self.aim = want.normalize();
        }
        // "The player shoots forward with space at 16 units per second every 0.25 seconds, using "shot", with the shoot sound." [player_shoot/countdown]
        self.shot_timer -= dt;
        if cx.key("space") && self.shot_timer <= 0.0 {
            self.shot_timer = 0.25;
            let dir = self.aim;
            let shot = cx.spawn_sprite("shot", &["shot"], "shot", cx.pos("player") + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
            cx.set_solid(&shot, false);
            cx.set_vel(&shot, dir * 16.0);
            cx.play_sound("shoot");
        }
        // "Each zombie moves toward the player at 2.3 units per second, speeding up by 1.2 percent per second." [chase/hoisted]
        let speed = 2.3 * (1.0 + self.clock * 0.012);
        for n in cx.find_tag("zombie") {
            let to = player - cx.pos(&n);
            cx.set_vel(&n, vec2(to.x, to.y).normalize_or_zero() * speed);
        }
        // "Each ghost moves toward the player at 1.8 units per second within 9 units." [chase/hoisted]
        for n in cx.find_tag("ghost") {
            let to = player - cx.pos(&n);
            if to.length() > 9.0 {
                cx.set_vel(&n, Vec3::ZERO);
                continue;
            }
            cx.set_vel(&n, vec2(to.x, to.y).normalize_or_zero() * 1.8);
        }
        // "Each blob wanders at 2 units per second, turning every 1.5 seconds." [wander/shared_timer]
        self.wander_blob -= dt;
        if self.wander_blob <= 0.0 {
            self.wander_blob += 1.5;
            for n in cx.find_tag("blob") {
                let a = cx.random(0.0, std::f32::consts::TAU);
                cx.set_vel(&n, vec2(a.cos(), a.sin()) * 2.0);
            }
        }
        // "When the time reaches 60 seconds, the player wins "YOU SURVIVED"." [reaches/edge]
        let reached = self.clock >= 60.0;
        if reached && !self.time_60_0 {
            // the player wins "YOU SURVIVED" [win/inline]
            if self.status == Status::Playing {
                self.status = Status::Won;
                cx.play_sound("win");
                cx.text_at("banner", "YOU SURVIVED - PRESS R", 0.3, 0.4, 44.0);
            }
        }
        self.time_60_0 = reached;
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
        // "When a shot hits a zombie, destroy both, add 1 to the score and play the explosion sound." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "zombie") {
            // destroy both [destroy/despawn]
            cx.destroy(a);
            cx.destroy(b);
            // add 1 to the score [score/add]
            self.score += 1;
            // play the explosion sound [sound/effect]
            cx.play_sound("explosion");
        }
        // "When a shot hits a blob, destroy both and add 2 points." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "blob") {
            // destroy both [destroy/despawn]
            cx.destroy(a);
            cx.destroy(b);
            // add 2 points [score/add]
            self.score += 2;
        }
        // "When the player touches an enemy, the player loses a life, safe for 1.5 seconds." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "enemy") {
            // the player loses a life, safe for 1.5 seconds [hurt/inline]
            if self.safe <= 0.0 {
                self.lives -= 1;
                cx.play_sound("hurt");
                if self.lives <= 0 {
                    if self.status == Status::Playing {
                        self.status = Status::Lost;
                        cx.play_sound("lose");
                        cx.text_at("banner", "GAME OVER - PRESS R", 0.3, 0.4, 44.0);
                    }
                } else {
                    self.safe = 1.5;
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
