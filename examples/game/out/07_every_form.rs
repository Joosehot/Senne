//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;
use std::collections::BTreeMap;

struct DroidGame {
    status: Status,
    aim: Vec3,
    air_jumped: bool,
    jump_buffer: f32,
    coyote: f32,
    shot_timer: f32,
    shot_timer_2: f32,
    shot_timer_3: f32,
    clock: f32,
    patrol_slime: BTreeMap<String, (f32, f32)>,
    wander_blob: f32,
    wander_spider: f32,
    march_alien: f32,
    most_alien: f32,
    wave: i64,
    ghost_fire: f32,
    saw_fire: f32,
    lives: i64,
    score: i64,
    timer: f32,
    safe: f32,
    no_alien_left: bool,
    score_50: bool,
    time_60_0: bool,
    start: Vec3,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            aim: Vec3::X,
            air_jumped: false,
            jump_buffer: 0.0,
            coyote: 0.0,
            shot_timer: 0.0,
            shot_timer_2: 0.0,
            shot_timer_3: 0.0,
            clock: 0.0,
            patrol_slime: BTreeMap::new(),
            wander_blob: 0.0,
            wander_spider: 0.0,
            march_alien: 1.0,
            most_alien: 0.0,
            wave: 1,
            ghost_fire: 2.0,
            saw_fire: 3.0,
            lives: 3,
            score: 0,
            timer: 1.6,
            safe: 0.0,
            no_alien_left: false,
            score_50: false,
            time_60_0: false,
            start: Vec3::ZERO,
        }
    }
}

impl DroidGame {
    fn hurt(&mut self, cx: &mut Ctx, safe: f32) {
        if self.safe > 0.0 {
            return;
        }
        self.lives -= 1;
        cx.play_sound("hurt");
        if self.lives <= 0 {
            self.lose(cx, "GAME OVER - PRESS R");
        } else {
            self.safe = safe;
        }
    }

    fn lose(&mut self, cx: &mut Ctx, banner: &str) {
        if self.status != Status::Playing {
            return;
        }
        self.status = Status::Lost;
        cx.play_sound("lose");
        cx.text_at("banner", banner, 0.3, 0.4, 44.0);
    }

    fn win(&mut self, cx: &mut Ctx, banner: &str) {
        if self.status != Status::Playing {
            return;
        }
        self.status = Status::Won;
        cx.play_sound("win");
        cx.text_at("banner", banner, 0.3, 0.4, 44.0);
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        self.start = cx.pos("player");
        // "Animate each slime with "slime_1" and "slime_2" at 4 frames per second." [animate/engine]
        for n in cx.find_tag("slime") {
            cx.animate_sprite(&n, &["slime_1", "slime_2"], 4.0);
        }
        // "At the start, the player has 3 lives, play the music "C4 E4 G4 C5" at 132 beats per minute as a square wave and show "COINS {score} / 14" at the top left in "#ffffff"." [start/init]
        // the player has 3 lives [lives/set]
        self.lives = 3;
        // play the music "C4 E4 G4 C5" at 132 beats per minute as a square wave [music/play]
        cx.play_music(132.0, "C4 E4 G4 C5", "square");
        // show "COINS {score} / 14" at the top left in "#ffffff" [show/every_frame]
        cx.text_at("text_0", &format!("COINS {} / 14", self.score), 0.02, 0.03, 24.0);
        cx.text_color("text_0", vec3(1.0, 1.0, 1.0));
        // "At the start, show "{lives} {wave} {time} {countdown 60} {left coin}" at the bottom." [start/init]
        // show "{lives} {wave} {time} {countdown 60} {left coin}" at the bottom [show/every_frame]
        cx.text_at("text_1", &format!("{} {} {} {} {}", self.lives, self.wave, self.clock as i64, (60.0 - self.clock).max(0.0).ceil() as i64, cx.count_tag("coin")), 0.4, 0.92, 24.0);
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, the player has 3 lives, play the music "C4 E4 G4 C5" at 132 beats per minute as a square wave and show "COINS {score} / 14" at the top left in "#ffffff"." [show/every_frame]
        cx.text_at("text_0", &format!("COINS {} / 14", self.score), 0.02, 0.03, 24.0);
        // "At the start, show "{lives} {wave} {time} {countdown 60} {left coin}" at the bottom." [show/every_frame]
        cx.text_at("text_1", &format!("{} {} {} {} {}", self.lives, self.wave, self.clock as i64, (60.0 - self.clock).max(0.0).ceil() as i64, cx.count_tag("coin")), 0.4, 0.92, 24.0);
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
        // "Move the player sideways with the arrow keys at 7.5 units per second and face where it moves." [move_player/axis]
        let want = cx.axis_x() * 7.5;
        cx.set_vel_x("player", want);
        if want != 0.0 {
            cx.set_flip_x("player", want < 0.0);
        }
        if want != 0.0 {
            self.aim = vec2(want.signum(), 0.0);
        }
        // "Move the player with the arrow keys at 7 units per second." [move_player/normalized]
        let want = vec2(cx.axis_x(), cx.axis_y()).normalize_or_zero() * 7.0;
        cx.set_vel_x("player", want.x);
        cx.set_vel_y("player", want.y);
        if want != Vec3::ZERO {
            self.aim = want.normalize();
        }
        // "Move the player across the ground with the arrow keys at 7 units per second, smoothly, within 12 units." [move_player/normalized]
        let want = vec2(cx.axis_x(), cx.axis_y()).normalize_or_zero() * 7.0;
        let ease = 1.0 - (-12.0 * dt).exp();
        let v = cx.vel("player");
        cx.set_vel_x("player", v.x + (want.x - v.x) * ease);
        cx.set_vel_z("player", v.z + (-want.y - v.z) * ease);
        if want != Vec3::ZERO {
            self.aim = vec3(want.x, 0.0, -want.y).normalize();
        }
        let p = cx.pos("player");
        if p.x.abs() > 12.0 {
            cx.set_pos("player", vec3(p.x.clamp(-12.0, 12.0), p.y, p.z));
            cx.set_vel_x("player", 0.0);
        }
        // "The player jumps with space at 14.5 units per second, twice at 12 units per second, higher while held, with the jump sound." [jump/buffered]
        if cx.grounded("player") {
            self.coyote = 0.1;
            self.air_jumped = false;
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
            cx.set_vel_y("player", 14.5);
            cx.play_sound("jump");
        } else if cx.key_pressed("space") && self.coyote <= 0.0 && !self.air_jumped {
            self.air_jumped = true;
            self.jump_buffer = 0.0;
            cx.set_vel_y("player", 12.0);
            cx.play_sound("jump");
        }
        let rise = cx.vel("player").y;
        if rise > 0.0 && !cx.key("space") {
            cx.set_vel_y("player", rise - 40.0 * dt);
        }
        // "The player shoots up with space at 22 units per second every 0.3 seconds, using "shot", with the shoot sound." [player_shoot/countdown]
        self.shot_timer -= dt;
        if cx.key("space") && self.shot_timer <= 0.0 {
            self.shot_timer = 0.3;
            let dir = vec2(0.0, 1.0);
            let shot = cx.spawn_sprite("shot", &["shot"], "shot", cx.pos("player") + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
            cx.set_solid(&shot, false);
            cx.set_vel(&shot, dir * 22.0);
            cx.play_sound("shoot");
        }
        // "The player shoots forward across the ground with x at 15 units per second." [player_shoot/countdown]
        self.shot_timer_2 -= dt;
        if cx.key("x") && self.shot_timer_2 <= 0.0 {
            self.shot_timer_2 = 0.25;
            let dir = self.aim;
            let shot = cx.spawn_sprite("shot", &["shot"], "shot", cx.pos("player") + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
            cx.set_solid(&shot, false);
            cx.set_vel(&shot, dir * 15.0);
        }
        // "The player shoots down with z at 10 units per second every 0.5 seconds." [player_shoot/countdown]
        self.shot_timer_3 -= dt;
        if cx.key("z") && self.shot_timer_3 <= 0.0 {
            self.shot_timer_3 = 0.5;
            let dir = vec2(0.0, -1.0);
            let shot = cx.spawn_sprite("shot", &["shot"], "shot", cx.pos("player") + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
            cx.set_solid(&shot, false);
            cx.set_vel(&shot, dir * 10.0);
        }
        // "Each zombie moves toward the player at 2.3 units per second." [chase/hoisted]
        for n in cx.find_tag("zombie") {
            let to = player - cx.pos(&n);
            cx.set_vel(&n, vec2(to.x, to.y).normalize_or_zero() * 2.3);
        }
        // "Each zombie moves toward the player sideways at 2 units per second within 9 units, speeding up by 1.2 percent per second." [chase/hoisted]
        let speed = 2.0 * (1.0 + self.clock * 0.012);
        for n in cx.find_tag("zombie") {
            let to = player - cx.pos(&n);
            if to.length() > 9.0 {
                cx.set_vel_x(&n, 0.0);
                continue;
            }
            if to.x.abs() > 0.1 {
                cx.set_vel_x(&n, to.x.signum() * speed);
                cx.set_flip_x(&n, to.x < 0.0);
            } else {
                cx.set_vel_x(&n, 0.0);
            }
        }
        // "Each skeleton moves toward the player across the ground at 2 units per second." [chase/hoisted]
        for n in cx.find_tag("skeleton") {
            let to = player - cx.pos(&n);
            let dir = vec3(to.x, 0.0, to.z).normalize_or_zero();
            cx.set_vel_x(&n, dir.x * 2.0);
            cx.set_vel_z(&n, dir.z * 2.0);
        }
        // "Each slime patrols 3 units to each side at 2.2 units per second." [patrol/bounds]
        for n in cx.find_tag("slime") {
            let x = cx.pos(&n).x;
            let (home, dir) = *self.patrol_slime.entry(n.clone()).or_insert((x, 1.0));
            let dir = if x > home + 3.0 { -1.0 } else if x < home - 3.0 { 1.0 } else { dir };
            cx.set_vel_x(&n, dir * 2.2);
            cx.set_flip_x(&n, dir < 0.0);
            self.patrol_slime.insert(n, (home, dir));
        }
        // "Each bat flies in a wave 1.2 units high at 2 cycles per second." [fly/velocity]
        let t = cx.time();
        for n in cx.find_tag("bat") {
            cx.set_vel_y(&n, (t * 12.566371).cos() * 15.079646);
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
        // "Each spider wanders across the ground at 2 units per second, turning every 1 seconds." [wander/shared_timer]
        self.wander_spider -= dt;
        if self.wander_spider <= 0.0 {
            self.wander_spider += 1.0;
            for n in cx.find_tag("spider") {
                let a = cx.random(0.0, std::f32::consts::TAU);
                cx.set_vel_x(&n, a.cos() * 2.0);
                cx.set_vel_z(&n, a.sin() * 2.0);
            }
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
        // "Each ghost shoots at the player at 6 units per second every 2 seconds within 10 units, using "bomb"." [enemy_shoot/hoisted]
        self.ghost_fire -= dt;
        if self.ghost_fire <= 0.0 {
            self.ghost_fire += 2.0;
            for n in cx.find_tag("ghost") {
                let from = cx.pos(&n);
                let to = player - from;
                if to.length() > 10.0 {
                    continue;
                }
                let dir = to.normalize_or_zero();
                let shot = cx.spawn_sprite("bomb", &["bomb"], "bomb", from + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
                cx.set_solid(&shot, false);
                cx.set_vel(&shot, dir * 6.0);
            }
        }
        // "Each saw shoots down at 4 units per second every 3 seconds, with the laser sound." [enemy_shoot/fixed]
        self.saw_fire -= dt;
        if self.saw_fire <= 0.0 {
            self.saw_fire += 3.0;
            for n in cx.find_tag("saw") {
                let from = cx.pos(&n);
                let dir = vec2(0.0, -1.0);
                let shot = cx.spawn_sprite("bomb", &["bomb"], "bomb", from + dir * 0.8, Vec3::splat(0.4), BodyKind::Kinematic);
                cx.set_solid(&shot, false);
                cx.set_vel(&shot, dir * 4.0);
                cx.play_sound("laser");
            }
        }
        // "Animate the player with "hero_1" and "hero_2" at 8 frames per second while it walks." [animate/clock]
        let frame = ["hero_1", "hero_2"][(cx.time() * 8.0) as usize % 2];
        let v = cx.vel("player");
        let walking = v.x.abs() > 0.1 && cx.grounded("player");
        cx.set_sprite("player", if walking { frame } else { "hero_1" });
        // "Every frame, show "T {time}" at the top right." [every_frame/update]
        // show "T {time}" at the top right [show/once]
        cx.text_at("text_2", &format!("T {}", self.clock as i64), 0.78, 0.03, 24.0);
        // "Every 1.6 seconds, a random alien shoots down at 7 units per second." [every/countdown]
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.timer += 1.6;
            // a random alien shoots down at 7 units per second [random_shot/pick]
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
        // "When no aliens are left, start the next wave of aliens." [none_left/edge]
        let none = cx.count_tag("alien") == 0;
        if none && !self.no_alien_left {
            // start the next wave of aliens [next_wave/respawn]
            self.wave += 1;
            cx.respawn_tag("alien");
        }
        self.no_alien_left = none;
        // "When the score reaches 50, the player wins "LEVEL CLEAR"." [reaches/edge]
        let reached = self.score >= 50;
        if reached && !self.score_50 {
            // the player wins "LEVEL CLEAR" [win/helper]
            self.win(cx, "LEVEL CLEAR - PRESS R");
        }
        self.score_50 = reached;
        // "When the time reaches 60 seconds, the player loses "TIME UP"." [reaches/edge]
        let reached = self.clock >= 60.0;
        if reached && !self.time_60_0 {
            // the player loses "TIME UP" [lose/helper]
            self.lose(cx, "TIME UP - PRESS R");
        }
        self.time_60_0 = reached;
        // "When the player falls below -12, the player loses a life and goes back to the start." [falls_below/check]
        if cx.pos("player").y < -12.0 {
            // the player loses a life [hurt/helper]
            self.hurt(cx, 1.0);
            // goes back to the start [back_to_start/reset]
            cx.set_pos("player", self.start);
            cx.set_vel("player", Vec3::ZERO);
        }
        // "When an alien falls below -6, destroy it and the player loses." [falls_below/check]
        for n in cx.find_tag("alien") {
            if cx.pos(&n).y < -6.0 {
                // destroy it [destroy/despawn]
                cx.destroy(&n);
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
        // "When the player lands on a slime, squash it, bounce the player up at 11 units per second and play the jump animation on the player." [lands_on/above]
        if a == "player" && cx.has_tag(b, "slime") && cx.pos(a).y - cx.pos(b).y > 0.25 {
            if cx.vel(a).y <= 0.0 {
                // squash it [squash/tween]
                cx.squash(b, 0.3);
                // bounce the player up at 11 units per second [bounce/set_vel]
                cx.set_vel_y("player", 11.0);
                // play the jump animation on the player [play_anim/oneshot]
                cx.play_anim("player", "jump");
            }
            return;
        }
        // "When the player touches a coin, destroy it, add 1 to the score and play the coin sound." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "coin") {
            // destroy it [destroy/despawn]
            cx.destroy(b);
            // add 1 to the score [score/add]
            self.score += 1;
            // play the coin sound [sound/effect]
            cx.play_sound("coin");
        }
        // "When the player touches the door and no coins are left, the player wins." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "door") && cx.count_tag("coin") == 0 {
            // the player wins [win/helper]
            self.win(cx, "YOU WIN! - PRESS R");
        }
        // "When a shot hits an alien, destroy the shot, destroy the alien and add 10 points." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "alien") {
            // destroy the shot [destroy/despawn]
            cx.destroy(a);
            // destroy the alien [destroy/despawn]
            cx.destroy(b);
            // add 10 points [score/add]
            self.score += 10;
        }
        // "When a shot hits a bat, destroy both." [hit/has_tag]
        if cx.has_tag(a, "shot") && cx.has_tag(b, "bat") {
            // destroy both [destroy/despawn]
            cx.destroy(a);
            cx.destroy(b);
        }
        // "When the player touches a slime, the player loses a life, safe for 1.5 seconds." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "slime") {
            // the player loses a life, safe for 1.5 seconds [hurt/helper]
            self.hurt(cx, 1.5);
        }
        // "When a bomb hits the player, destroy the bombs, squash the player and show "OUCH" in the middle." [hit/has_tag]
        if cx.has_tag(a, "bomb") && b == "player" {
            // destroy the bombs [destroy/despawn]
            cx.destroy_tag("bomb");
            // squash the player [squash/tween]
            cx.squash("player", 0.3);
            // show "OUCH" in the middle [show/once]
            cx.text_at("text_3", "OUCH", 0.36, 0.45, 24.0);
        }
        // "When the player touches a spike, the player loses "GAME OVER"." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "spike") {
            // the player loses "GAME OVER" [lose/helper]
            self.lose(cx, "GAME OVER - PRESS R");
        }
    }

    fn status(&self) -> Status {
        self.status
    }
}

fn main() {
    droid::run::<DroidGame>(concat!(env!("CARGO_MANIFEST_DIR"), "/game"));
}
