//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;
use std::collections::BTreeMap;

struct DroidGame {
    status: Status,
    lives: i64,
    score: i64,
    air_jumped: bool,
    jump_buffer: f32,
    coyote: f32,
    patrol_slime: BTreeMap<String, (f32, f32)>,
    safe: f32,
    start: Vec3,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            lives: 3,
            score: 0,
            air_jumped: false,
            jump_buffer: 0.0,
            coyote: 0.0,
            patrol_slime: BTreeMap::new(),
            safe: 0.0,
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
            if self.status == Status::Playing {
                self.status = Status::Lost;
                cx.play_sound("lose");
                cx.text_at("banner", "GAME OVER - PRESS R", 0.3, 0.4, 44.0);
            }
        } else {
            self.safe = safe;
        }
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        self.start = cx.pos("player");
        // "At the start, the player has 3 lives, play the music "C4 E4 G4 C5 G4 E4 C4 -" at 132 beats per minute as a square wave, show "COINS {score} / 14" at the top left and show "LIVES {lives}" at the top right." [start/init]
        // the player has 3 lives [lives/set]
        self.lives = 3;
        // play the music "C4 E4 G4 C5 G4 E4 C4 -" at 132 beats per minute as a square wave [music/play]
        cx.play_music(132.0, "C4 E4 G4 C5 G4 E4 C4 -", "square");
        // show "COINS {score} / 14" at the top left [show/every_frame]
        cx.text_at("text_0", &format!("COINS {} / 14", self.score), 0.02, 0.03, 24.0);
        // show "LIVES {lives}" at the top right [show/every_frame]
        cx.text_at("text_1", &format!("LIVES {}", self.lives), 0.78, 0.03, 24.0);
        // "Animate each slime with "slime_1" and "slime_2" at 4 frames per second." [animate/engine]
        for n in cx.find_tag("slime") {
            cx.animate_sprite(&n, &["slime_1", "slime_2"], 4.0);
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, the player has 3 lives, play the music "C4 E4 G4 C5 G4 E4 C4 -" at 132 beats per minute as a square wave, show "COINS {score} / 14" at the top left and show "LIVES {lives}" at the top right." [show/every_frame]
        cx.text_at("text_0", &format!("COINS {} / 14", self.score), 0.02, 0.03, 24.0);
        // "At the start, the player has 3 lives, play the music "C4 E4 G4 C5 G4 E4 C4 -" at 132 beats per minute as a square wave, show "COINS {score} / 14" at the top left and show "LIVES {lives}" at the top right." [show/every_frame]
        cx.text_at("text_1", &format!("LIVES {}", self.lives), 0.78, 0.03, 24.0);
        if self.status != Status::Playing {
            return;
        }
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
        // "Animate the player with "hero_1" and "hero_2" at 8 frames per second while it walks." [animate/clock]
        let frame = ["hero_1", "hero_2"][(cx.time() * 8.0) as usize % 2];
        let v = cx.vel("player");
        let walking = v.x.abs() > 0.1 && cx.grounded("player");
        cx.set_sprite("player", if walking { frame } else { "hero_1" });
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
        // "When the player falls below -12, the player loses a life and goes back to the start." [falls_below/check]
        if cx.pos("player").y < -12.0 {
            // the player loses a life [hurt/helper]
            self.hurt(cx, 1.0);
            // goes back to the start [back_to_start/reset]
            cx.set_pos("player", self.start);
            cx.set_vel("player", Vec3::ZERO);
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When the player lands on a slime, destroy it, bounce the player up at 11 units per second, squash the player and play the bounce sound." [lands_on/above]
        if a == "player" && cx.has_tag(b, "slime") && cx.pos(a).y - cx.pos(b).y > 0.25 {
            if cx.vel(a).y <= 0.0 {
                // destroy it [destroy/despawn]
                cx.destroy(b);
                // bounce the player up at 11 units per second [bounce/set_vel]
                cx.set_vel_y("player", 11.0);
                // squash the player [squash/tween]
                cx.squash("player", 0.3);
                // play the bounce sound [sound/effect]
                cx.play_sound("bounce");
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
        // "When the player touches a slime, the player loses a life, safe for 1.5 seconds." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "slime") {
            // the player loses a life, safe for 1.5 seconds [hurt/helper]
            self.hurt(cx, 1.5);
        }
        // "When the player touches a bat, the player loses a life, safe for 1.5 seconds." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "bat") {
            // the player loses a life, safe for 1.5 seconds [hurt/helper]
            self.hurt(cx, 1.5);
        }
        // "When the player touches the flag and no coins are left, the player wins "LEVEL CLEAR"." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "flag") && cx.count_tag("coin") == 0 {
            // the player wins "LEVEL CLEAR" [win/inline]
            if self.status == Status::Playing {
                self.status = Status::Won;
                cx.play_sound("win");
                cx.text_at("banner", "LEVEL CLEAR - PRESS R", 0.3, 0.4, 44.0);
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
