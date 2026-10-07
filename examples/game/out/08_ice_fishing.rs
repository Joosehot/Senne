//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;
use std::collections::BTreeMap;

struct DroidGame {
    status: Status,
    score: i64,
    clock: f32,
    line_hook: BTreeMap<String, String>,
    hooked: BTreeMap<String, String>,
    patrol_perch: BTreeMap<String, f32>,
    patrol_roach: BTreeMap<String, f32>,
    patrol_pike: BTreeMap<String, f32>,
    hooked_at: BTreeMap<String, f32>,
    flash_middle: f32,
    shy: BTreeMap<String, f32>,
    comebacks: Vec<(f32, String, bool)>,
    bottom_fish: BTreeMap<String, f32>,
    score_50: bool,
    time_90_0: bool,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            score: 0,
            clock: 0.0,
            line_hook: BTreeMap::new(),
            hooked: BTreeMap::new(),
            patrol_perch: BTreeMap::new(),
            patrol_roach: BTreeMap::new(),
            patrol_pike: BTreeMap::new(),
            hooked_at: BTreeMap::new(),
            flash_middle: 0.0,
            shy: BTreeMap::new(),
            comebacks: Vec::new(),
            bottom_fish: BTreeMap::new(),
            score_50: false,
            time_90_0: false,
        }
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f" and show the shout meter at the bottom." [start/init]
        // show "SCORE {score} / 50" at the top left in "#16324f" [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {} / 50", self.score), 0.02, 0.03, 24.0);
        cx.text_color("text_0", vec3(0.086, 0.196, 0.31));
        // show "TIME {countdown 90}" at the top right in "#16324f" [show/every_frame]
        cx.text_at("text_1", &format!("TIME {}", (90.0 - self.clock).max(0.0).ceil() as i64), 0.78, 0.03, 24.0);
        cx.text_color("text_1", vec3(0.086, 0.196, 0.31));
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f" and show the shout meter at the bottom." [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {} / 50", self.score), 0.02, 0.03, 24.0);
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f" and show the shout meter at the bottom." [show/every_frame]
        cx.text_at("text_1", &format!("TIME {}", (90.0 - self.clock).max(0.0).ceil() as i64), 0.78, 0.03, 24.0);
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f" and show the shout meter at the bottom." [meter/every_frame]
        let shout = cx.voice();
        let bars = ((shout * 20.0).round() as usize).min(20);
        cx.text_at("text_2", &format!("SHOUT {}{}", "#".repeat(bars), "-".repeat(20 - bars)), 0.4, 0.92, 24.0);
        cx.text_color("text_2", if shout >= 0.25 { vec3(0.45, 1.0, 0.45) } else { vec3(0.7, 0.75, 0.8) });
        if self.status != Status::Playing {
            return;
        }
        self.clock += dt;
        // the message at the middle goes when its time is up
        if self.flash_middle > 0.0 && self.clock >= self.flash_middle {
            cx.remove_text("flash_middle");
            self.flash_middle = 0.0;
        }
        // scene things that come back once their time is up
        let due: Vec<(String, bool)> = self.comebacks.iter().filter(|c| c.0 <= self.clock).map(|c| (c.1.clone(), c.2)).collect();
        self.comebacks.retain(|c| c.0 > self.clock);
        for (name, side) in due {
            if let Some(n) = cx.respawn(&name) {
                if side {
                    let x = if cx.random(0.0, 1.0) < 0.5 { -18.0 } else { 18.0 };
                    let y = cx.pos(&n).y;
                    cx.set_pos(&n, vec2(x, y));
                }
            }
        }
        // "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9." [hang/tracked]
        for n in cx.find_tag("hook") {
            if cx.pos(&n).y > -9.0 {
                cx.move_by(&n, vec2(0.0, -1.5 * dt));
            }
        }
        // "The hook rises while the player shouts or holds space, at up to 8 units per second as loud as the shout." [rise/linear]
        let shout = cx.voice();
        let lift = if shout >= 0.25 { shout * 8.0 } else { 0.0 };
        let lift = if cx.key("space") { 8.0 } else { lift };
        if lift > 0.0 {
            for n in cx.find_tag("hook") {
                cx.move_by(&n, vec2(0.0, lift * dt));
            }
        }
        // "Each perch swims within 16 units at 2 units per second." [patrol/bounds]
        for n in cx.find_tag("perch") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            let x = cx.pos(&n).x;
            let dir = *self.patrol_perch.entry(n.clone()).or_insert(if x > 0.0 { -1.0 } else { 1.0 });
            let dir = if x > 16.0 { -1.0 } else if x < -16.0 { 1.0 } else { dir };
            cx.set_vel_x(&n, dir * 2.0);
            cx.set_flip_x(&n, dir < 0.0);
            self.patrol_perch.insert(n, dir);
        }
        // "Each roach swims within 16 units at 3 units per second." [patrol/bounds]
        for n in cx.find_tag("roach") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            let x = cx.pos(&n).x;
            let dir = *self.patrol_roach.entry(n.clone()).or_insert(if x > 0.0 { -1.0 } else { 1.0 });
            let dir = if x > 16.0 { -1.0 } else if x < -16.0 { 1.0 } else { dir };
            cx.set_vel_x(&n, dir * 3.0);
            cx.set_flip_x(&n, dir < 0.0);
            self.patrol_roach.insert(n, dir);
        }
        // "Each pike swims within 16 units at 1.5 units per second." [patrol/bounds]
        for n in cx.find_tag("pike") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            let x = cx.pos(&n).x;
            let dir = *self.patrol_pike.entry(n.clone()).or_insert(if x > 0.0 { -1.0 } else { 1.0 });
            let dir = if x > 16.0 { -1.0 } else if x < -16.0 { 1.0 } else { dir };
            cx.set_vel_x(&n, dir * 1.5);
            cx.set_flip_x(&n, dir < 0.0);
            self.patrol_pike.insert(n, dir);
        }
        // "Each hooked perch follows its hook, fighting and pulling it down at 2 units per second in bursts." [hooked/copy]
        for (fish, hook) in self.hooked.clone() {
            if !cx.has_tag(&fish, "perch") {
                continue;
            }
            let t = self.clock - self.hooked_at.get(&fish).copied().unwrap_or(self.clock);
            let surge = t % 1.6 < 0.9;
            if surge {
                cx.move_by(&hook, vec2(0.0, -2.0 * dt));
            }
            let wild = if surge { 1.0 } else { 0.3 };
            let phase = t * 15.707964;
            let dart = phase.sin() * 0.45 * wild;
            // it jerks the hook with it (the line springs it back)
            cx.move_by(&hook, vec2(phase.cos() * 4.2411504 * wild * dt, 0.0));
            let at = cx.pos(&hook) + vec2(dart, -0.7);
            cx.set_pos(&fish, at);
            cx.set_flip_x(&fish, phase.cos() < 0.0);
            cx.set_vel(&fish, Vec3::ZERO);
        }
        // "Each hooked roach follows its hook, fighting and pulling it down at 1.5 units per second in bursts." [hooked/copy]
        for (fish, hook) in self.hooked.clone() {
            if !cx.has_tag(&fish, "roach") {
                continue;
            }
            let t = self.clock - self.hooked_at.get(&fish).copied().unwrap_or(self.clock);
            let surge = t % 1.6 < 0.9;
            if surge {
                cx.move_by(&hook, vec2(0.0, -1.5 * dt));
            }
            let wild = if surge { 1.0 } else { 0.3 };
            let phase = t * 15.707964;
            let dart = phase.sin() * 0.45 * wild;
            // it jerks the hook with it (the line springs it back)
            cx.move_by(&hook, vec2(phase.cos() * 4.2411504 * wild * dt, 0.0));
            let at = cx.pos(&hook) + vec2(dart, -0.7);
            cx.set_pos(&fish, at);
            cx.set_flip_x(&fish, phase.cos() < 0.0);
            cx.set_vel(&fish, Vec3::ZERO);
        }
        // "Each hooked pike follows its hook, fighting and pulling it down at 5 units per second in bursts." [hooked/copy]
        for (fish, hook) in self.hooked.clone() {
            if !cx.has_tag(&fish, "pike") {
                continue;
            }
            let t = self.clock - self.hooked_at.get(&fish).copied().unwrap_or(self.clock);
            let surge = t % 1.6 < 0.9;
            if surge {
                cx.move_by(&hook, vec2(0.0, -5.0 * dt));
            }
            let wild = if surge { 1.0 } else { 0.3 };
            let phase = t * 15.707964;
            let dart = phase.sin() * 0.45 * wild;
            // it jerks the hook with it (the line springs it back)
            cx.move_by(&hook, vec2(phase.cos() * 4.2411504 * wild * dt, 0.0));
            let at = cx.pos(&hook) + vec2(dart, -0.7);
            cx.set_pos(&fish, at);
            cx.set_flip_x(&fish, phase.cos() < 0.0);
            cx.set_vel(&fish, Vec3::ZERO);
        }
        // "When a hooked perch reaches the hole, add 10 points, destroy it, play the splash sound, show "PERCH +10" in the middle for 1.5 seconds and a new perch comes in from the side after 3 seconds." [landed/scan]
        if let Some(top) = cx.find_tag("hole").first().map(|h| cx.pos(h)) {
            for (n, hook) in self.hooked.clone() {
                if cx.has_tag(&n, "perch") && cx.pos(&hook).y >= top.y - 0.3 {
                    self.hooked.remove(&n);
                    // add 10 points [score/add]
                    self.score += 10;
                    // destroy it [destroy/despawn]
                    cx.destroy(&n);
                    // play the splash sound [sound/effect]
                    cx.play_sound("splash");
                    // show "PERCH +10" in the middle for 1.5 seconds [show/once]
                    cx.text_at("flash_middle", "PERCH +10", 0.36, 0.45, 36.0);
                    self.flash_middle = self.clock + 1.5;
                    // a new perch comes in from the side after 3 seconds [come_back/queue]
                    self.comebacks.push((self.clock + 3.0, String::from(&n), true));
                }
            }
        }
        // "When a hooked roach reaches the hole, add 5 points, destroy it, play the splash sound, show "ROACH +5" in the middle for 1.5 seconds and a new roach comes in from the side after 2 seconds." [landed/scan]
        if let Some(top) = cx.find_tag("hole").first().map(|h| cx.pos(h)) {
            for (n, hook) in self.hooked.clone() {
                if cx.has_tag(&n, "roach") && cx.pos(&hook).y >= top.y - 0.3 {
                    self.hooked.remove(&n);
                    // add 5 points [score/add]
                    self.score += 5;
                    // destroy it [destroy/despawn]
                    cx.destroy(&n);
                    // play the splash sound [sound/effect]
                    cx.play_sound("splash");
                    // show "ROACH +5" in the middle for 1.5 seconds [show/once]
                    cx.text_at("flash_middle", "ROACH +5", 0.36, 0.45, 36.0);
                    self.flash_middle = self.clock + 1.5;
                    // a new roach comes in from the side after 2 seconds [come_back/queue]
                    self.comebacks.push((self.clock + 2.0, String::from(&n), true));
                }
            }
        }
        // "When a hooked pike reaches the hole, add 30 points, destroy it, play the splash sound, show "PIKE +30" in the middle for 1.5 seconds and a new pike comes in from the side after 6 seconds." [landed/scan]
        if let Some(top) = cx.find_tag("hole").first().map(|h| cx.pos(h)) {
            for (n, hook) in self.hooked.clone() {
                if cx.has_tag(&n, "pike") && cx.pos(&hook).y >= top.y - 0.3 {
                    self.hooked.remove(&n);
                    // add 30 points [score/add]
                    self.score += 30;
                    // destroy it [destroy/despawn]
                    cx.destroy(&n);
                    // play the splash sound [sound/effect]
                    cx.play_sound("splash");
                    // show "PIKE +30" in the middle for 1.5 seconds [show/once]
                    cx.text_at("flash_middle", "PIKE +30", 0.36, 0.45, 36.0);
                    self.flash_middle = self.clock + 1.5;
                    // a new pike comes in from the side after 6 seconds [come_back/queue]
                    self.comebacks.push((self.clock + 6.0, String::from(&n), true));
                }
            }
        }
        // "When a hooked fish stays at the bottom for 3 seconds, it gets away, play the splash sound and show "IT GOT AWAY!" in the middle for 1.5 seconds." [at_bottom/timer]
        for (n, hook) in self.hooked.clone() {
            if !cx.has_tag(&n, "fish") {
                continue;
            }
            let down = cx.pos(&hook).y <= -8.8;
            let held = if down { self.bottom_fish.get(&n).copied().unwrap_or(0.0) + dt } else { 0.0 };
            self.bottom_fish.insert(n.clone(), held);
            if held >= 3.0 {
                self.bottom_fish.remove(&n);
                // it gets away [get_away/unhook]
                self.hooked.remove(&n);
                self.shy.insert(String::from(&n), self.clock + 3.0);
                // play the splash sound [sound/effect]
                cx.play_sound("splash");
                // show "IT GOT AWAY!" in the middle for 1.5 seconds [show/once]
                cx.text_at("flash_middle", "IT GOT AWAY!", 0.36, 0.45, 36.0);
                self.flash_middle = self.clock + 1.5;
            }
        }
        // "When the score reaches 50, the player wins "GREAT CATCH"." [reaches/edge]
        let reached = self.score >= 50;
        if reached && !self.score_50 {
            // the player wins "GREAT CATCH" [win/inline]
            if self.status == Status::Playing {
                self.status = Status::Won;
                cx.play_sound("win");
                cx.text_at("banner", "GREAT CATCH - PRESS R", 0.3, 0.4, 44.0);
            }
        }
        self.score_50 = reached;
        // "When the time reaches 90 seconds, the player loses "TIME UP"." [reaches/edge]
        let reached = self.clock >= 90.0;
        if reached && !self.time_90_0 {
            // the player loses "TIME UP" [lose/inline]
            if self.status == Status::Playing {
                self.status = Status::Lost;
                cx.play_sound("lose");
                cx.text_at("banner", "TIME UP - PRESS R", 0.3, 0.4, 44.0);
            }
        }
        self.time_90_0 = reached;
        // "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9." [hang/tracked]
        if let Some(top) = cx.find_tag("hole").first().map(|h| cx.pos(h)) {
            for n in cx.find_tag("hook") {
                let p = cx.pos(&n);
                let y = p.y.clamp(-9.0, top.y);
                let x = top.x + (p.x - top.x) * (-6.0 * dt).exp();
                cx.set_pos(&n, vec2(x, y));
                let line = match self.line_hook.get(&n) {
                    Some(l) if cx.exists(l) => l.clone(),
                    _ => {
                        let l = cx.spawn_shape(&format!("{n}_line"), &["line"], "box", top, Vec3::ONE, vec3(0.92, 0.92, 0.88), BodyKind::None);
                        self.line_hook.insert(n.clone(), l.clone());
                        l
                    }
                };
                cx.set_pos(&line, vec3((top.x + x) * 0.5, (top.y + y) * 0.5, -0.1));
                cx.set_scale(&line, vec3(0.06, vec2(x - top.x, y - top.y).length().max(0.06), 0.06));
                cx.set_rotation(&line, vec3(0.0, 0.0, (x - top.x).atan2(top.y - y).to_degrees()));
            }
        }
    }

    fn on_collision(&mut self, cx: &mut Ctx, a: &str, b: &str) {
        if self.status != Status::Playing {
            return;
        }
        // "When a hook touches a fish and no fish is hooked, the fish bites, squash it, play the hit sound and show "FISH ON! SHOUT!" in the middle for 1.5 seconds." [hit/has_tag]
        if cx.has_tag(a, "hook") && cx.has_tag(b, "fish") && !self.hooked.keys().any(|n| cx.has_tag(n, "fish")) && self.shy.get(b).map_or(true, |t| *t <= self.clock) {
            // the fish bites [bite/map]
            self.hooked.insert(b.to_string(), a.to_string());
            self.hooked_at.insert(b.to_string(), self.clock);
            // squash it [squash/tween]
            cx.squash(b, 0.3);
            // play the hit sound [sound/effect]
            cx.play_sound("hit");
            // show "FISH ON! SHOUT!" in the middle for 1.5 seconds [show/once]
            cx.text_at("flash_middle", "FISH ON! SHOUT!", 0.36, 0.45, 36.0);
            self.flash_middle = self.clock + 1.5;
        }
    }

    fn status(&self) -> Status {
        self.status
    }
}

fn main() {
    droid::run::<DroidGame>(concat!(env!("CARGO_MANIFEST_DIR"), "/game"));
}
