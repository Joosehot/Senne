//! DroidGame: written by Senne from the game's sentences (behavior.txt).
//! Each block is commented with the sentence that asked for it and the [pattern/variant]
//! that wrote it. Change the sentences, not this file.

use droid::prelude::*;
use std::collections::BTreeMap;

struct DroidGame {
    status: Status,
    level: usize,
    choosing: bool,
    choose_secs: f32,
    score: i64,
    clock: f32,
    flash_middle: f32,
    hooked: BTreeMap<String, String>,
    line_hook: BTreeMap<String, String>,
    jigged: BTreeMap<String, f32>,
    rest_hook: BTreeMap<String, f32>,
    swim_perch: BTreeMap<String, (f32, f32, f32, f32, f32, f32)>,
    swim_roach: BTreeMap<String, (f32, f32, f32, f32, f32, f32)>,
    swim_pike: BTreeMap<String, (f32, f32, f32, f32, f32, f32)>,
    shy: BTreeMap<String, f32>,
    hooked_at: BTreeMap<String, f32>,
    beat_hook: BTreeMap<String, (f32, f32)>,
    comebacks: Vec<(f32, String, bool)>,
    bottom_fish: BTreeMap<String, f32>,
    score_50: bool,
    time_90_0: bool,
}

impl Default for DroidGame {
    fn default() -> Self {
        DroidGame {
            status: Status::Playing,
            level: 1,
            choosing: true,
            choose_secs: 0.0,
            score: 0,
            clock: 0.0,
            flash_middle: 0.0,
            hooked: BTreeMap::new(),
            line_hook: BTreeMap::new(),
            jigged: BTreeMap::new(),
            rest_hook: BTreeMap::new(),
            swim_perch: BTreeMap::new(),
            swim_roach: BTreeMap::new(),
            swim_pike: BTreeMap::new(),
            shy: BTreeMap::new(),
            hooked_at: BTreeMap::new(),
            beat_hook: BTreeMap::new(),
            comebacks: Vec::new(),
            bottom_fish: BTreeMap::new(),
            score_50: false,
            time_90_0: false,
        }
    }
}

impl Game for DroidGame {
    fn init(&mut self, cx: &mut Ctx) {
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f", show "JIG: UP / W" in the middle for 6 seconds and show the shout meter at the bottom." [start/init]
        // show "SCORE {score} / 50" at the top left in "#16324f" [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {} / 50", self.score), 0.02, 0.03, 24.0);
        cx.text_color("text_0", vec3(0.086, 0.196, 0.31));
        // show "TIME {countdown 90}" at the top right in "#16324f" [show/every_frame]
        cx.text_at("text_1", &format!("TIME {}", ([120.0, 90.0, 70.0][self.level] - self.clock).max(0.0).ceil() as i64), 0.78, 0.03, 24.0);
        cx.text_color("text_1", vec3(0.086, 0.196, 0.31));
        // show "JIG: UP / W" in the middle for 6 seconds [show/once]
        cx.text_at("flash_middle", "JIG: UP / W", 0.36, 0.45, 36.0);
        self.flash_middle = self.clock + 6.0;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32) {
        // "When the player presses r, restart the game." [key_press/pressed]
        if cx.key_pressed("r") {
            // restart the game [restart/request]
            cx.restart();
            return;
        }
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f", show "JIG: UP / W" in the middle for 6 seconds and show the shout meter at the bottom." [show/every_frame]
        cx.text_at("text_0", &format!("SCORE {} / 50", self.score), 0.02, 0.03, 24.0);
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f", show "JIG: UP / W" in the middle for 6 seconds and show the shout meter at the bottom." [show/every_frame]
        cx.text_at("text_1", &format!("TIME {}", ([120.0, 90.0, 70.0][self.level] - self.clock).max(0.0).ceil() as i64), 0.78, 0.03, 24.0);
        // "At the start, show "SCORE {score} / 50" at the top left in "#16324f", show "TIME {countdown 90}" at the top right in "#16324f", show "JIG: UP / W" in the middle for 6 seconds and show the shout meter at the bottom." [meter/bar]
        let shout = cx.voice();
        let th = [0.15, 0.25, 0.35][self.level];
        let fill = shout.clamp(0.0, 1.0);
        // green when quiet, yellow where a shout starts to count, red at full voice
        let color = if fill < th { vec3(0.3, 0.9, 0.35) + (vec3(1.0, 0.85, 0.2) - vec3(0.3, 0.9, 0.35)) * (fill / th) } else { vec3(1.0, 0.85, 0.2) + (vec3(1.0, 0.3, 0.25) - vec3(1.0, 0.85, 0.2)) * ((fill - th) / (1.0 - th)) };
        // it pulses while a fish is on
        let pulse = if self.hooked.is_empty() { 0.0 } else { 0.5 + 0.5 * (cx.time() * 15.707964).sin() };
        let frame = vec3(0.85, 0.92, 1.0) + (vec3(1.0, 0.3, 0.25) - vec3(0.85, 0.92, 1.0)) * pulse;
        cx.rect_at("text_2_frame", 0.324, 0.854, 0.352, 0.072, frame);
        cx.rect_at("text_2_back", 0.33, 0.86, 0.34, 0.06, vec3(0.05, 0.09, 0.15));
        cx.rect_at("text_2_fill", 0.33, 0.86, 0.34 * fill, 0.06, color);
        cx.rect_at("text_2_mark", 0.327 + 0.34 * th, 0.854, 0.006, 0.072, vec3(1.0, 1.0, 1.0));
        cx.text_at("text_2", "SHOUT!", 0.33, 0.8, 34.0);
        cx.text_color("text_2", frame);
        if self.status != Status::Playing {
            return;
        }
        // "The player chooses easy, normal or hard at the start with 1, 2 and 3, or normal with space or after 6 seconds." [choose/prompt]
        // the game waits until the player picks a level
        if self.choosing {
            self.choose_secs += dt;
            let pick = if cx.key_pressed("1") { Some(0) } else if cx.key_pressed("2") { Some(1) } else if cx.key_pressed("3") { Some(2) } else if cx.key_pressed("space") { Some(1) } else if self.choose_secs >= 6.0 { Some(1) } else { None };
            match pick {
                Some(l) => {
                    self.level = l;
                    self.choosing = false;
                    cx.remove_text("choose");
                }
                None => {
                    cx.text_at("choose", "1 EASY  2 NORMAL  3 HARD", 0.27, 0.3, 40.0);
                    return;
                }
            }
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
        // "The hook hangs by its eye on a line from the rod tip down through the hole, sinking at 1.5 units per second down to -8.3." [hang/tracked]
        for n in cx.find_tag("hook") {
            if cx.pos(&n).y > -8.3 {
                cx.move_by(&n, vec2(0.0, -1.5 * dt));
            }
        }
        // "The player jigs the hook with the up arrow or w, lifting it 0.8 units, and it flutters back down." [jig/snap]
        let jig = cx.key_pressed("up") || cx.key_pressed("w");
        for n in cx.find_tag("hook") {
            let y = cx.pos(&n).y;
            // where it rests: the lowest it has been since the jigging began
            let rest = match self.rest_hook.get(&n) {
                Some(r) if self.jigged.get(&n).is_some_and(|t| self.clock - *t < 2.5) => r.min(y),
                _ => y,
            };
            self.rest_hook.insert(n.clone(), rest);
            if jig && y - rest < 0.8 && !self.hooked.values().any(|h| *h == n) {
                self.jigged.insert(n.clone(), self.clock);
                // up to the top of the band, no higher
                cx.move_by(&n, vec2(0.0, rest + 0.8 - y));
            }
            if let Some(t0) = self.jigged.get(&n).copied() {
                let t = self.clock - t0;
                // it flutters back down: a sway from side to side that fades
                if t < 2.5 {
                    let fade = 1.0 - t / 2.5;
                    cx.move_by(&n, vec2((t * 13.823009).cos() * 1.658761 * fade * dt, 0.0));
                }
            }
        }
        // "The hook rises while the player shouts or holds space, at up to 8 units per second as loud as the shout." [rise/linear]
        let shout = cx.voice();
        let lift = if shout >= [0.15, 0.25, 0.35][self.level] { shout * 8.0 } else { 0.0 };
        let lift = if cx.key("space") { 8.0 } else { lift };
        if lift > 0.0 {
            for n in cx.find_tag("hook") {
                cx.move_by(&n, vec2(0.0, lift * dt));
            }
        }
        // "Animate each hook with "hook_jig_1", "hook_jig_2", "hook_jig_3", "hook_jig_4", "hook_jig_5", "hook_jig_6", "hook_jig_7" and "hook_jig_8" at 12 frames per second while it is jigged." [animate/clock]
        let frame = ["hook_jig_1", "hook_jig_2", "hook_jig_3", "hook_jig_4", "hook_jig_5", "hook_jig_6", "hook_jig_7", "hook_jig_8"][(cx.time() * 12.0) as usize % 8];
        for n in cx.find_tag("hook") {
            cx.set_sprite(&n, if self.jigged.get(&n).is_some_and(|t| self.clock - *t < 2.5) { frame } else { "hook_jig_1" });
        }
        // "Each perch swims in a school within 15 units at 2 units per second, darting, keeping its depth within 1.5 units and nosing toward the hook within 4 units." [swim/steer]
        let near = cx.find_tag("hook").first().map(|h| cx.pos(h));
        let school: Vec<(String, Vec3)> = cx.find_tag("perch").into_iter().filter(|n| !self.hooked.contains_key(n)).map(|n| { let p = cx.pos(&n); (n, p) }).collect();
        for (n, p) in school.clone() {
            let (home, dir, vx, beat, lunge, wait) = *self.swim_perch.entry(n.clone()).or_insert((p.y, if p.x > 0.0 { -1.0 } else { 1.0 }, 0.0, -1.0, 0.0, 0.0));
            // its own clock, from a seed made of its name
            let seed = (n.bytes().fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b))) % 1000) as f32 / 1000.0;
            let t = self.clock * (0.85 + 0.3 * seed) + seed * 7.0;
            // darting: a quick burst, then a glide that slows into a short pause
            let cycle = t / 1.8;
            let turn = cycle.floor() != beat && ((cycle.floor() + seed * 31.0).sin() * 437.585).fract().abs() < 0.3;
            let beat = cycle.floor();
            let dir = if turn { -dir } else { dir };
            let burst = cycle.fract() * 1.8 < 0.35;
            let go = if burst { 5.2 } else { 0.0 };
            let rate = if burst { 9.0 } else { 2.2 };
            // the school: turn with its first fish, close in on the others, keep loose spacing
            let (mut mid, mut count, mut push, mut lead) = (Vec3::ZERO, 0.0, Vec3::ZERO, None);
            for (o, q) in &school {
                let d = *q - p;
                if d.length() < 5.0 {
                    if lead.is_none() {
                        lead = Some(o.clone());
                    }
                    if *o != n {
                        mid += *q;
                        count += 1.0;
                        if d.length() < 1.6 {
                            push -= d.normalize_or_zero() * (1.6 - d.length());
                        }
                    }
                }
            }
            let dir = lead.filter(|o| *o != n).and_then(|o| self.swim_perch.get(&o)).map_or(dir, |o| o.1);
            let pull = if count > 0.0 { (mid * (1.0 / count) - p) * 0.35 } else { Vec3::ZERO } + push * 1.5;
            let dir = if p.x > 15.0 { -1.0 } else if p.x < -15.0 { 1.0 } else { dir };
            // curious: noses toward it when it is near
            let nose = near.filter(|h| (*h - p).length() < 4.0);
            let dir = nose.map_or(dir, |h| if h.x < p.x { -1.0 } else { 1.0 });
            let go = nose.map_or(go, |h| ((h.x - p.x).abs() * 1.5).min(1.0));
            let vx = vx + (dir * go - vx) * (1.0 - (-rate * dt).exp());
            let ty = (home + (t * 0.35 + seed * 6.283).sin() * 1.05).clamp(home - 1.5, home + 1.5);
            let ty = nose.map_or(ty, |h| h.y.clamp(home - 1.5, home + 1.5));
            let vy = (ty - p.y) * 1.2;
            cx.set_vel(&n, vec2(vx + pull.x, vy + pull.y));
            if vx.abs() > 0.15 {
                cx.set_flip_x(&n, vx < 0.0);
            }
            self.swim_perch.insert(n, (home, dir, vx, beat, lunge, wait));
        }
        // "Each roach swims in a shoal within 15 units at 1.5 units per second, drifting, turning smoothly, in a wave 0.6 units high and keeping its depth within 1 unit." [swim/steer]
        let school: Vec<(String, Vec3)> = cx.find_tag("roach").into_iter().filter(|n| !self.hooked.contains_key(n)).map(|n| { let p = cx.pos(&n); (n, p) }).collect();
        for (n, p) in school.clone() {
            let (home, dir, vx, beat, lunge, wait) = *self.swim_roach.entry(n.clone()).or_insert((p.y, if p.x > 0.0 { -1.0 } else { 1.0 }, 0.0, -1.0, 0.0, 0.0));
            // its own clock, from a seed made of its name
            let seed = (n.bytes().fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b))) % 1000) as f32 / 1000.0;
            let t = self.clock * (0.85 + 0.3 * seed) + seed * 7.0;
            // drifting: calm, the speed swells and eases
            let go = 1.5 * (1.0 + 0.25 * (t * 0.6).sin());
            let rate = 1.2;
            // the school: turn with its first fish, close in on the others, keep loose spacing
            let (mut mid, mut count, mut push, mut lead) = (Vec3::ZERO, 0.0, Vec3::ZERO, None);
            for (o, q) in &school {
                let d = *q - p;
                if d.length() < 5.0 {
                    if lead.is_none() {
                        lead = Some(o.clone());
                    }
                    if *o != n {
                        mid += *q;
                        count += 1.0;
                        if d.length() < 1.6 {
                            push -= d.normalize_or_zero() * (1.6 - d.length());
                        }
                    }
                }
            }
            let dir = lead.filter(|o| *o != n).and_then(|o| self.swim_roach.get(&o)).map_or(dir, |o| o.1);
            let pull = if count > 0.0 { (mid * (1.0 / count) - p) * 0.35 } else { Vec3::ZERO } + push * 1.5;
            let dir = if p.x > 15.0 { -1.0 } else if p.x < -15.0 { 1.0 } else { dir };
            let vx = vx + (dir * go - vx) * (1.0 - (-rate * dt).exp());
            let ty = (home + (t * 0.35 + seed * 6.283).sin() * 0.7 + (t * 1.5707964).sin() * 0.3).clamp(home - 1.0, home + 1.0);
            let vy = (ty - p.y) * 1.2;
            cx.set_vel(&n, vec2(vx + pull.x, vy + pull.y));
            if vx.abs() > 0.15 {
                cx.set_flip_x(&n, vx < 0.0);
            }
            self.swim_roach.insert(n, (home, dir, vx, beat, lunge, wait));
        }
        // "Each pike swims within 15 units at 1 unit per second, lurking, keeping its depth within 0.5 units and lunging at the hook within 6 units at 9 units per second." [swim/steer]
        let near = cx.find_tag("hook").first().map(|h| cx.pos(h));
        for n in cx.find_tag("pike") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            let p = cx.pos(&n);
            let (home, dir, vx, beat, lunge, wait) = *self.swim_pike.entry(n.clone()).or_insert((p.y, if p.x > 0.0 { -1.0 } else { 1.0 }, 0.0, -1.0, 0.0, 0.0));
            // its own clock, from a seed made of its name
            let seed = (n.bytes().fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b))) % 1000) as f32 / 1000.0;
            let t = self.clock * (0.85 + 0.3 * seed) + seed * 7.0;
            // lurking: hovers almost still, the fins drifting it slowly; now and then it turns
            let cycle = t / 5.0;
            let turn = cycle.floor() != beat && ((cycle.floor() + seed * 31.0).sin() * 437.585).fract().abs() < 0.3;
            let beat = cycle.floor();
            let dir = if turn { -dir } else { dir };
            let go = 1.0 * (0.1 + 0.9 * (0.5 + 0.5 * (t * 0.8).sin()));
            let rate = 6.0;
            let dir = if p.x > 15.0 { -1.0 } else if p.x < -15.0 { 1.0 } else { dir };
            let vx = vx + (dir * go - vx) * (1.0 - (-rate * dt).exp());
            let ty = (home + (t * 0.35 + seed * 6.283).sin() * 0.35).clamp(home - 0.5, home + 0.5);
            let vy = (ty - p.y) * 1.2;
            // lunging: a sudden dash at it when it comes near, then a glide back
            let start = lunge <= 0.0 && self.clock >= wait && near.is_some_and(|h| (h - p).length() < 6.0);
            let (lunge, wait) = if start { (0.7, self.clock + 3.2) } else { ((lunge - dt).max(0.0), wait) };
            let dash = near.filter(|_| lunge > 0.0).map(|h| (h - p).normalize_or_zero() * 9.0);
            let (vx, vy) = dash.map_or((vx, vy), |d| (d.x, d.y));
            cx.set_vel(&n, vec2(vx, vy));
            if vx.abs() > 0.15 {
                cx.set_flip_x(&n, vx < 0.0);
            }
            self.swim_pike.insert(n, (home, dir, vx, beat, lunge, wait));
        }
        // "Fish are drawn to a jigged hook within 5 units at 2.5 units per second." [drawn/steer]
        let lure = cx.find_tag("hook").into_iter().find(|h| self.jigged.get(h).is_some_and(|t| self.clock - *t < 2.5) && !self.hooked.values().any(|k| k == h)).map(|h| cx.pos(&h));
        if let Some(h) = lure {
            for n in cx.find_tag("fish") {
                if self.hooked.contains_key(&n) {
                    continue;
                }
                if self.shy.get(&n).is_some_and(|t| *t > self.clock) {
                    continue;
                }
                let p = cx.pos(&n);
                let d = h - p;
                if d.length() < 5.0 {
                    let want = d.normalize_or_zero() * 2.5 * (d.length() / 1.0).min(1.0);
                    let v = cx.vel(&n);
                    let v = v + (want - v) * (1.0 - (-6.0 * dt).exp());
                    cx.set_vel(&n, vec2(v.x, v.y));
                    // it turns to face the lure
                    if d.x.abs() > 0.15 {
                        cx.set_flip_x(&n, d.x < 0.0);
                    }
                }
            }
        }
        // "Animate each perch with "perch_swim_1", "perch_swim_2", "perch_swim_3", "perch_swim_4", "perch_swim_5", "perch_swim_6", "perch_swim_7" and "perch_swim_8" at 10 frames per second." [animate/clock]
        let frame = ["perch_swim_1", "perch_swim_2", "perch_swim_3", "perch_swim_4", "perch_swim_5", "perch_swim_6", "perch_swim_7", "perch_swim_8"][(cx.time() * 10.0) as usize % 8];
        for n in cx.find_tag("perch") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            cx.set_sprite(&n, frame);
        }
        // "Animate each roach with "roach_swim_1", "roach_swim_2", "roach_swim_3", "roach_swim_4", "roach_swim_5", "roach_swim_6", "roach_swim_7" and "roach_swim_8" at 12 frames per second." [animate/clock]
        let frame = ["roach_swim_1", "roach_swim_2", "roach_swim_3", "roach_swim_4", "roach_swim_5", "roach_swim_6", "roach_swim_7", "roach_swim_8"][(cx.time() * 12.0) as usize % 8];
        for n in cx.find_tag("roach") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            cx.set_sprite(&n, frame);
        }
        // "Animate each pike with "pike_swim_1", "pike_swim_2", "pike_swim_3", "pike_swim_4", "pike_swim_5", "pike_swim_6", "pike_swim_7" and "pike_swim_8" at 8 frames per second." [animate/clock]
        let frame = ["pike_swim_1", "pike_swim_2", "pike_swim_3", "pike_swim_4", "pike_swim_5", "pike_swim_6", "pike_swim_7", "pike_swim_8"][(cx.time() * 8.0) as usize % 8];
        for n in cx.find_tag("pike") {
            if self.hooked.contains_key(&n) {
                continue;
            }
            cx.set_sprite(&n, frame);
        }
        // "Animate each perch with "perch_struggle_1", "perch_struggle_2", "perch_struggle_3", "perch_struggle_4", "perch_struggle_5" and "perch_struggle_6" at 12 frames per second while it is hooked." [animate/clock]
        let frame = ["perch_struggle_1", "perch_struggle_2", "perch_struggle_3", "perch_struggle_4", "perch_struggle_5", "perch_struggle_6"][(cx.time() * 12.0) as usize % 6];
        for n in cx.find_tag("perch") {
            if self.hooked.contains_key(&n) {
                cx.set_sprite(&n, frame);
            }
        }
        // "Animate each roach with "roach_struggle_1", "roach_struggle_2", "roach_struggle_3", "roach_struggle_4", "roach_struggle_5" and "roach_struggle_6" at 12 frames per second while it is hooked." [animate/clock]
        let frame = ["roach_struggle_1", "roach_struggle_2", "roach_struggle_3", "roach_struggle_4", "roach_struggle_5", "roach_struggle_6"][(cx.time() * 12.0) as usize % 6];
        for n in cx.find_tag("roach") {
            if self.hooked.contains_key(&n) {
                cx.set_sprite(&n, frame);
            }
        }
        // "Animate each pike with "pike_struggle_1", "pike_struggle_2", "pike_struggle_3", "pike_struggle_4", "pike_struggle_5" and "pike_struggle_6" at 10 frames per second while it is hooked." [animate/clock]
        let frame = ["pike_struggle_1", "pike_struggle_2", "pike_struggle_3", "pike_struggle_4", "pike_struggle_5", "pike_struggle_6"][(cx.time() * 10.0) as usize % 6];
        for n in cx.find_tag("pike") {
            if self.hooked.contains_key(&n) {
                cx.set_sprite(&n, frame);
            }
        }
        // "Each hooked perch follows its hook, fighting and pulling it down at 2 units per second in bursts." [hooked/copy]
        for (fish, hook) in self.hooked.clone() {
            if !cx.has_tag(&fish, "perch") {
                continue;
            }
            let t = self.clock - self.hooked_at.get(&fish).copied().unwrap_or(self.clock);
            let surge = t % 1.6 < [0.675, 0.9, 1.125][self.level];
            if surge {
                cx.move_by(&hook, vec2(0.0, -[1.2, 2.0, 2.7][self.level] * dt));
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
            let surge = t % 1.6 < [0.675, 0.9, 1.125][self.level];
            if surge {
                cx.move_by(&hook, vec2(0.0, -[0.9, 1.5, 2.025][self.level] * dt));
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
            let surge = t % 1.6 < [0.675, 0.9, 1.125][self.level];
            if surge {
                cx.move_by(&hook, vec2(0.0, -[3.0, 5.0, 6.75][self.level] * dt));
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
        // "When the hook is jigged within 2 units of a fish and no fish is hooked, the fish bites with a chance of 40 percent for a perch, 20 percent for a roach, 5 percent for a pike and 60 percent for a lunging pike, more for a steady rhythm, less for frantic jigging and less the farther it is, squash it, play the hit sound and show "FISH ON! SHOUT!" in the middle for 1.5 seconds." [jig_near/each_jig]
        for h in cx.find_tag("hook") {
            if self.jigged.get(&h).copied() != Some(self.clock) {
                continue;
            }
            let lure = cx.pos(&h);
            // the rhythm: the time since the last jig, and the gap before that
            let (last, before) = self.beat_hook.get(&h).copied().unwrap_or((f32::NEG_INFINITY, 0.0));
            let gap = self.clock - last;
            self.beat_hook.insert(h.clone(), (self.clock, gap));
            let steady = (0.35..=1.2).contains(&gap) && (gap - before).abs() <= 0.25;
            let frantic = gap < 0.3;
            for n in cx.find_tag("fish") {
                if self.hooked.keys().any(|k| cx.has_tag(k, "fish")) {
                    break;
                }
                if self.hooked.contains_key(&n) {
                    continue;
                }
                if self.shy.get(&n).is_some_and(|t| *t > self.clock) {
                    continue;
                }
                let d = (cx.pos(&n) - lure).length();
                if d >= 2.0 {
                    continue;
                }
                let near = 1.0 - d / 2.0;
                // the fish bites with a chance of 40 percent for a perch, 20 percent for a roach, 5 percent for a pike, 60 percent for a lunging pike, more for a steady rhythm, less for frantic jigging, less the farther it is [bite/map]
                // it bites by chance, once per jig
                let chance = if cx.has_tag(n.as_str(), "pike") && self.swim_pike.get(n.as_str()).is_some_and(|s| s.4 > 0.0) { 0.6 } else if cx.has_tag(n.as_str(), "perch") { 0.4 } else if cx.has_tag(n.as_str(), "roach") { 0.2 } else if cx.has_tag(n.as_str(), "pike") { 0.05 } else { 0.0 };
                let chance = if steady { chance * 1.6 } else { chance };
                let chance = if frantic { chance * 0.15 } else { chance };
                let chance = chance * near;
                let chance = chance * [1.4, 1.0, 0.7][self.level];
                if cx.random(0.0, 1.0) >= chance {
                    continue;
                }
                self.hooked.insert(n.as_str().to_string(), h.as_str().to_string());
                self.hooked_at.insert(n.as_str().to_string(), self.clock);
                // squash it [squash/tween]
                cx.squash(n.as_str(), 0.3);
                // play the hit sound [sound/effect]
                cx.play_sound("hit");
                // show "FISH ON! SHOUT!" in the middle for 1.5 seconds [show/once]
                cx.text_at("flash_middle", "FISH ON! SHOUT!", 0.36, 0.45, 36.0);
                self.flash_middle = self.clock + 1.5;
            }
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
            let down = cx.pos(&hook).y <= -8.1;
            let held = if down { self.bottom_fish.get(&n).copied().unwrap_or(0.0) + dt } else { 0.0 };
            self.bottom_fish.insert(n.clone(), held);
            if held >= [4.8, 3.0, 1.95][self.level] {
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
        let reached = self.clock >= [120.0, 90.0, 70.0][self.level];
        if reached && !self.time_90_0 {
            // the player loses "TIME UP" [lose/inline]
            if self.status == Status::Playing {
                self.status = Status::Lost;
                cx.play_sound("lose");
                cx.text_at("banner", "TIME UP - PRESS R", 0.3, 0.4, 44.0);
            }
        }
        self.time_90_0 = reached;
        // "The hook hangs by its eye on a line from the rod tip down through the hole, sinking at 1.5 units per second down to -8.3." [hang/tracked]
        if let Some(top) = cx.find_tag("rod_tip").first().map(|h| cx.pos(h)) {
            if let Some(gate) = cx.find_tag("hole").first().map(|h| cx.pos(h)) {
                for n in cx.find_tag("hook") {
                    let p = cx.pos(&n);
                    let y = p.y.clamp(-8.3, gate.y);
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
                    // the line ends at its eye: the top middle of the hook
                    let y = y + cx.scale(&n).y * 0.5;
                    cx.set_pos(&line, vec3((top.x + x) * 0.5, (top.y + y) * 0.5, -0.1));
                    cx.set_scale(&line, vec3(0.06, vec2(x - top.x, y - top.y).length().max(0.06), 0.06));
                    cx.set_rotation(&line, vec3(0.0, 0.0, (x - top.x).atan2(top.y - y).to_degrees()));
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
