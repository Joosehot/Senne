//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;

struct DroidGame {
    status: Status,
    score: i64,
    jump_buffer: f32,
    coyote: f32,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            score: 0,
            jump_buffer: 0.0,
            coyote: 0.0,
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
        // "At the start, show "KEYS {score} / 3" at the top left and show "KEYS LEFT {left key}" at the top left." [start/init]
        // show "KEYS {score} / 3" at the top left [show/every_frame]
        cx.text_at("text_0", &format!("KEYS {} / 3", self.score), 0.02, 0.03, 24.0);
        // show "KEYS LEFT {left key}" at the top left [show/every_frame]
        cx.text_at("text_1", &format!("KEYS LEFT {}", cx.count_tag("key")), 0.02, 0.08, 24.0);
        // "Animate each skeleton with "skeleton_1" and "skeleton_2" at 5 frames per second." [animate/engine]
        for n in cx.find_tag("skeleton") {
            cx.animate_sprite(&n, &["skeleton_1", "skeleton_2"], 5.0);
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, show "KEYS {score} / 3" at the top left and show "KEYS LEFT {left key}" at the top left." [show/every_frame]
        cx.text_at("text_0", &format!("KEYS {} / 3", self.score), 0.02, 0.03, 24.0);
        // "At the start, show "KEYS {score} / 3" at the top left and show "KEYS LEFT {left key}" at the top left." [show/every_frame]
        cx.text_at("text_1", &format!("KEYS LEFT {}", cx.count_tag("key")), 0.02, 0.08, 24.0);
        if self.status != Status::Playing {
            return;
        }
        // "Move the player sideways with the arrow keys at 6.5 units per second and face where it moves." [move_player/axis]
        let want = cx.axis_x() * 6.5;
        cx.set_vel_x("player", want);
        if want != 0.0 {
            cx.set_flip_x("player", want < 0.0);
        }
        // "The player jumps with space at 13 units per second with the jump sound." [jump/buffered]
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
            cx.set_vel_y("player", 13.0);
            cx.play_sound("jump");
        }
        // "Each skeleton moves toward the player sideways at 2 units per second within 6 units." [chase/per_loop]
        for n in cx.find_tag("skeleton") {
            let to = cx.pos("player") - cx.pos(&n);
            if to.length() > 6.0 {
                cx.set_vel_x(&n, 0.0);
                continue;
            }
            if to.x.abs() > 0.1 {
                cx.set_vel_x(&n, to.x.signum() * 2.0);
                cx.set_flip_x(&n, to.x < 0.0);
            } else {
                cx.set_vel_x(&n, 0.0);
            }
        }
        // "When the player falls below -10, the player loses." [falls_below/check]
        if cx.pos("player").y < -10.0 {
            // the player loses [lose/helper]
            self.lose(cx, "GAME OVER - PRESS R");
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When the player touches a key, destroy it, add 1 to the score and play the pickup sound." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "key") {
            // destroy it [destroy/despawn]
            cx.destroy(b);
            // add 1 to the score [score/add]
            self.score += 1;
            // play the pickup sound [sound/effect]
            cx.play_sound("pickup");
        }
        // "When the player touches the exit and no keys are left, the player wins "ESCAPED"." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "exit") && cx.count_tag("key") == 0 {
            // the player wins "ESCAPED" [win/inline]
            if self.status == Status::Playing {
                self.status = Status::Won;
                cx.play_sound("win");
                cx.text_at("banner", "ESCAPED - PRESS R", 0.3, 0.4, 44.0);
            }
        }
        // "When the player touches the exit, show "FIND THE KEYS FIRST" in the middle in "#ffcc00"." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "exit") {
            // show "FIND THE KEYS FIRST" in the middle in "#ffcc00" [show/once]
            cx.text_at("text_2", "FIND THE KEYS FIRST", 0.36, 0.45, 24.0);
            cx.text_color("text_2", vec3(1.0, 0.8, 0.0));
        }
        // "When the player touches a skeleton, the player loses." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "skeleton") {
            // the player loses [lose/helper]
            self.lose(cx, "GAME OVER - PRESS R");
        }
        // "When the player touches a spike, the player loses." [touch/has_tag]
        if a == "player" && cx.has_tag(b, "spike") {
            // the player loses [lose/helper]
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
