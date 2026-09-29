//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;
use std::collections::BTreeMap;

struct DroidGame {
    status: Status,
    jump_buffer: f32,
    coyote: f32,
    fly_bat: BTreeMap<String, (f32, f32)>,
    wander_blob: BTreeMap<String, f32>,
    score: i64,
    text_0_shown: String,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            jump_buffer: 0.0,
            coyote: 0.0,
            fly_bat: BTreeMap::new(),
            wander_blob: BTreeMap::new(),
            score: 0,
            text_0_shown: String::new(),
        }
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        // "Quickly, at the start, show "SCORE {score}" at the top left." [start/init]
        // show "SCORE {score}" at the top left [show/on_change]
        cx.text_at("text_0", &format!("SCORE {}", self.score), 0.02, 0.03, 24.0);
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "Quickly, at the start, show "SCORE {score}" at the top left." [show/on_change]
        let text = format!("SCORE {}", self.score);
        if text != self.text_0_shown {
            cx.text_at("text_0", &text, 0.02, 0.03, 24.0);
            self.text_0_shown = text;
        }
        if self.status != Status::Playing {
            return;
        }
        let player = cx.pos("player");
        // "Simply move the player with the arrow keys at 7 units per second." [move_player/raw]
        let want = vec2(cx.axis_x(), cx.axis_y()) * 7.0;
        cx.set_vel_x("player", want.x);
        cx.set_vel_y("player", want.y);
        // "Safely, the player jumps with space at 12 units per second." [jump/buffered]
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
            cx.set_vel_y("player", 12.0);
        }
        // "Quickly, each zombie moves toward the player at 2 units per second." [chase/hoisted]
        for n in cx.find_tag("zombie") {
            let to = player - cx.pos(&n);
            cx.set_vel(&n, vec2(to.x, to.y).normalize_or_zero() * 2.0);
        }
        // "Simply, each slime patrols 2 units to each side at 1.5 units per second." [patrol/sine]
        let t = cx.time();
        for n in cx.find_tag("slime") {
            let v = (t * 0.75).cos() * 1.5;
            cx.set_vel_x(&n, v);
            cx.set_flip_x(&n, v < 0.0);
        }
        // "Safely, each bat flies in a wave 1 units high at 1.5 cycles per second." [fly/anchored]
        let t = cx.time();
        for n in cx.find_tag("bat") {
            let p = cx.pos(&n);
            let (home, t0) = *self.fly_bat.entry(n.clone()).or_insert((p.y, t));
            cx.set_pos(&n, vec3(p.x, home + ((t - t0) * 9.424778).sin() * 1.0, p.z));
        }
        // "Each blob wanders at 2 units per second, turning every 2 seconds, safely." [wander/own_timer]
        for n in cx.find_tag("blob") {
            let left = self.wander_blob.entry(n.clone()).or_insert(0.0);
            *left -= dt;
            if *left <= 0.0 {
                *left = cx.random(1.0, 3.0);
                let a = cx.random(0.0, std::f32::consts::TAU);
                cx.set_vel(&n, vec2(a.cos(), a.sin()) * 2.0);
            }
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When the player touches a coin, destroy it and add 1 to the score." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "coin") {
            // destroy it [destroy/despawn]
            cx.destroy(b);
            // add 1 to the score [score/add]
            self.score += 1;
        }
        // "When the player touches a zombie, simply, the player loses." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "zombie") {
            // the player loses [lose/inline]
            if self.status == Status::Playing {
                self.status = Status::Lost;
                cx.play_sound("lose");
                cx.text_at("banner", "GAME OVER", 0.3, 0.4, 44.0);
            }
        }
        // "When the player touches a slime, the player loses." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "slime") {
            // the player loses [lose/inline]
            if self.status == Status::Playing {
                self.status = Status::Lost;
                cx.play_sound("lose");
                cx.text_at("banner", "GAME OVER", 0.3, 0.4, 44.0);
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
