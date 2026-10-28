//! Banc Rust des effets, jumeau de tools/fidelity/fx_modules.cpp (scripts/compare_fx.py).
//! Usage: compare_fx_render SCENARIOS STIMULI_DIR OUT_DIR RATE
//! Même fichier de scénarios que le C++; sortie OUT_DIR/<nom>.f64 = (gauche, droite) f64 LE entrelacés.
use odin2_core::dsp::effects::{Chorus, Delay, EffectsChain, Flanger, Phaser, ZitaReverb};
use odin2_core::preset::{ChorusParams, DelayParams, FlangerParams, PhaserParams, ReverbParams};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

struct Cfg {
    name: String,
    kind: String,
    stimulus: String,
    base: HashMap<String, f64>,
    events: Vec<(usize, String, f64)>,
}
impl Cfg {
    fn f(&self, key: &str) -> f32 {
        *self.base.get(key).unwrap_or_else(|| panic!("clé absente: {key} ({})", self.name)) as f32
    }
    fn b(&self, key: &str) -> bool { self.f(key) > 0.5 }
    fn i(&self, key: &str) -> i32 { self.f(key) as i32 }
}

trait Runner {
    fn set(&mut self, key: &str, value: f32);
    fn process(&mut self, l: f32, r: f32) -> (f32, f32);
}

struct DelayRunner { d: Delay, m: [f32; 5], bpm: f32, sync: bool }
impl DelayRunner {
    fn new(c: &Cfg, rate: f32) -> Self {
        let mut d = Delay::new(rate);
        let m = [c.f("mod_time"), c.f("mod_fb"), c.f("mod_hp"), c.f("mod_dry"), c.f("mod_wet")];
        d.set_modulation(m[0], m[1], m[2], m[3], m[4]);
        d.set_ping_pong(c.b("pp"));
        d.set_feedback(c.f("fb"));
        d.set_highpass_freq(c.f("hp"));
        d.set_ducking(c.f("duck"));
        d.set_dry(c.f("dry"));
        d.set_wet(c.f("wet"));
        d.set_sync_ratio(c.f("num"), c.f("den"));
        let sync = c.b("sync");
        let bpm = c.f("bpm");
        if sync { d.set_delay_from_bpm(bpm); } else { d.set_delay_time(c.f("time")); }
        if c.b("reset") { d.reset(); }
        Self { d, m, bpm, sync }
    }
    fn push_mod(&mut self) { let m = self.m; self.d.set_modulation(m[0], m[1], m[2], m[3], m[4]); }
}
impl Runner for DelayRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "time" => self.d.set_delay_time(v),
            "fb" => self.d.set_feedback(v),
            "dry" => self.d.set_dry(v),
            "wet" => self.d.set_wet(v),
            "hp" => self.d.set_highpass_freq(v),
            "duck" => self.d.set_ducking(v),
            "pp" => self.d.set_ping_pong(v > 0.5),
            "bpm" => { self.bpm = v; if self.sync { self.d.set_delay_from_bpm(v); } }
            "mod_time" => { self.m[0] = v; self.push_mod(); }
            "mod_fb" => { self.m[1] = v; self.push_mod(); }
            "mod_hp" => { self.m[2] = v; self.push_mod(); }
            "mod_dry" => { self.m[3] = v; self.push_mod(); }
            "mod_wet" => { self.m[4] = v; self.push_mod(); }
            "reset" => self.d.reset(),
            _ => panic!("delay: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { self.d.process(l, r) }
}

struct ChorusRunner { c: Chorus, m: [f32; 4], bpm: f32, sync: bool }
impl ChorusRunner {
    fn new(cfg: &Cfg, rate: f32) -> Self {
        let mut c = Chorus::new(rate);
        let m = [cfg.f("mod_freq"), cfg.f("mod_amount"), cfg.f("mod_fb"), cfg.f("mod_drywet")];
        c.set_modulation(m[0], m[1], m[2], m[3]);
        c.set_amount(cfg.f("amount"));
        c.set_feedback(cfg.f("fb"));
        c.set_dry_wet(cfg.f("drywet"));
        c.set_sync_ratio(cfg.f("num"), cfg.f("den"));
        let sync = cfg.b("sync");
        let bpm = cfg.f("bpm");
        if sync { c.set_lfo_from_bpm(bpm); } else { c.set_lfo_freq(cfg.f("rate")); }
        if cfg.b("reset") { c.reset(); }
        if cfg.b("lforeset") { c.reset_lfo(); }
        Self { c, m, bpm, sync }
    }
    fn push_mod(&mut self) { let m = self.m; self.c.set_modulation(m[0], m[1], m[2], m[3]); }
}
impl Runner for ChorusRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "amount" => self.c.set_amount(v),
            "rate" => self.c.set_lfo_freq(v),
            "fb" => self.c.set_feedback(v),
            "drywet" => self.c.set_dry_wet(v),
            "bpm" => { self.bpm = v; if self.sync { self.c.set_lfo_from_bpm(v); } }
            "lforeset" => self.c.reset_lfo(),
            "mod_freq" => { self.m[0] = v; self.push_mod(); }
            "mod_amount" => { self.m[1] = v; self.push_mod(); }
            "mod_fb" => { self.m[2] = v; self.push_mod(); }
            "mod_drywet" => { self.m[3] = v; self.push_mod(); }
            _ => panic!("chorus: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { self.c.process_stereo(l, r) }
}

struct FlangerRunner { f: [Flanger; 2], m: [f32; 4], bpm: f32, sync: bool }
impl FlangerRunner {
    fn new(c: &Cfg, rate: f32) -> Self {
        let mut f = [Flanger::new(rate), Flanger::new(rate)];
        f[1].set_lfo_reset_pos(0.5);
        let m = [c.f("mod_freq"), c.f("mod_amount"), c.f("mod_fb"), c.f("mod_drywet")];
        let sync = c.b("sync");
        let bpm = c.f("bpm");
        for x in &mut f {
            x.set_modulation(m[0], m[1], m[2], m[3]);
            x.set_amount(c.f("amount"));
            x.set_feedback(c.f("fb"));
            x.set_dry_wet(c.f("drywet"));
            x.set_sync_ratio(c.f("num"), c.f("den"));
            if sync { x.set_lfo_from_bpm(bpm); } else { x.set_lfo_freq(c.f("rate")); }
            if c.b("reset") { x.reset(); }
            if c.b("lforeset") { x.reset_lfo(); }
        }
        Self { f, m, bpm, sync }
    }
    fn push_mod(&mut self) { let m = self.m; for x in &mut self.f { x.set_modulation(m[0], m[1], m[2], m[3]); } }
}
impl Runner for FlangerRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "amount" => for x in &mut self.f { x.set_amount(v) },
            "rate" => for x in &mut self.f { x.set_lfo_freq(v) },
            "fb" => for x in &mut self.f { x.set_feedback(v) },
            "drywet" => for x in &mut self.f { x.set_dry_wet(v) },
            "bpm" => { self.bpm = v; if self.sync { for x in &mut self.f { x.set_lfo_from_bpm(v) } } }
            "lforeset" => for x in &mut self.f { x.reset_lfo() },
            "mod_freq" => { self.m[0] = v; self.push_mod(); }
            "mod_amount" => { self.m[1] = v; self.push_mod(); }
            "mod_fb" => { self.m[2] = v; self.push_mod(); }
            "mod_drywet" => { self.m[3] = v; self.push_mod(); }
            _ => panic!("flanger: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { (self.f[0].process(l), self.f[1].process(r)) }
}

struct PhaserRunner { p: Phaser, m: [f32; 5], bpm: f32, sync: bool }
impl PhaserRunner {
    fn new(c: &Cfg, rate: f32) -> Self {
        let mut p = Phaser::new(rate);
        p.set_radius(c.f("radius"));
        let m = [c.f("mod_rate"), c.f("mod_amount"), c.f("mod_freq"), c.f("mod_fb"), c.f("mod_drywet")];
        p.set_modulation(m[0], m[1], m[2], m[3], m[4]);
        p.set_amount(c.f("amount"));
        p.set_feedback(c.f("fb"));
        p.set_base_freq(c.f("freq"));
        p.set_dry_wet(c.f("drywet"));
        p.set_sync_ratio(c.f("num"), c.f("den"));
        let sync = c.b("sync");
        let bpm = c.f("bpm");
        if sync { p.set_lfo_from_bpm(bpm); } else { p.set_lfo_freq(c.f("rate")); }
        if c.b("reset") { p.reset(); }
        if c.b("lforeset") { p.reset_lfo(); }
        Self { p, m, bpm, sync }
    }
    fn push_mod(&mut self) { let m = self.m; self.p.set_modulation(m[0], m[1], m[2], m[3], m[4]); }
}
impl Runner for PhaserRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "amount" => self.p.set_amount(v),
            "rate" => self.p.set_lfo_freq(v),
            "fb" => self.p.set_feedback(v),
            "freq" => self.p.set_base_freq(v),
            "drywet" => self.p.set_dry_wet(v),
            "bpm" => { self.bpm = v; if self.sync { self.p.set_lfo_from_bpm(v); } }
            "lforeset" => self.p.reset_lfo(),
            "mod_rate" => { self.m[0] = v; self.push_mod(); }
            "mod_amount" => { self.m[1] = v; self.push_mod(); }
            "mod_freq" => { self.m[2] = v; self.push_mod(); }
            "mod_fb" => { self.m[3] = v; self.push_mod(); }
            "mod_drywet" => { self.m[4] = v; self.push_mod(); }
            _ => panic!("phaser: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { self.p.process(l, r) }
}

struct ReverbRunner { z: ZitaReverb }
impl ReverbRunner {
    fn new(c: &Cfg, rate: f32) -> Self {
        let mut z = ZitaReverb::new(rate);
        z.set_delay(c.f("delay") / 1000.0);
        z.set_rtmid(c.f("mid"));
        z.set_fdamp(c.f("damp"));
        z.set_eq_gain(c.f("eqgain"));
        z.set_eq_freq(c.f("eqfreq"));
        z.set_mix(c.f("drywet"));
        if c.b("lowpar") {
            z.set_xover(c.f("xover"));
            z.set_rtlow(c.f("rtlow"));
        }
        if c.b("reset") { z.reset(); }
        Self { z }
    }
}
impl Runner for ReverbRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "delay" => self.z.set_delay(v / 1000.0),
            "mid" => self.z.set_rtmid(v),
            "damp" => self.z.set_fdamp(v),
            "eqgain" => self.z.set_eq_gain(v),
            "eqfreq" => self.z.set_eq_freq(v),
            "drywet" => self.z.set_mix(v),
            _ => panic!("reverb: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { self.z.process(l, r) }
}

struct ChainRunner { chain: EffectsChain }
impl ChainRunner {
    fn new(c: &Cfg, rate: f32) -> Self {
        let mut chain = EffectsChain::new(rate);
        chain.set_bpm(c.f("bpm"));
        let delay = DelayParams {
            on: c.b("delay_on"), time: c.f("delay_time"), feedback: c.f("delay_fb"), dry: c.f("delay_dry"),
            wet: c.f("delay_wet"), hp: c.f("delay_hp"), ducking: c.f("delay_duck"), pingpong: c.b("delay_pp"),
            sync: c.b("delay_sync"), sync_numerator: c.i("delay_num"), sync_denominator: c.i("delay_den"),
        };
        let phaser = PhaserParams {
            on: c.b("phaser_on"), frequency: c.f("phaser_freq"), feedback: c.f("phaser_fb"), rate: c.f("phaser_rate"),
            mod_amount: c.f("phaser_amount"), dry_wet: c.f("phaser_drywet"), reset: c.b("phaser_reset"),
            sync: c.b("phaser_sync"), sync_numerator: c.i("phaser_num"), sync_denominator: c.i("phaser_den"),
        };
        let flanger = FlangerParams {
            on: c.b("flanger_on"), amount: c.f("flanger_amount"), rate: c.f("flanger_rate"), feedback: c.f("flanger_fb"),
            dry_wet: c.f("flanger_drywet"), reset: c.b("flanger_reset"),
            sync: c.b("flanger_sync"), sync_numerator: c.i("flanger_num"), sync_denominator: c.i("flanger_den"),
        };
        let chorus = ChorusParams {
            on: c.b("chorus_on"), amount: c.f("chorus_amount"), rate: c.f("chorus_rate"), feedback: c.f("chorus_fb"),
            dry_wet: c.f("chorus_drywet"), reset: c.b("chorus_reset"),
            sync: c.b("chorus_sync"), sync_numerator: c.i("chorus_num"), sync_denominator: c.i("chorus_den"),
        };
        let reverb = ReverbParams {
            on: c.b("reverb_on"), delay: c.f("reverb_delay"), dry_wet: c.f("reverb_drywet"), mid_hall: c.f("reverb_mid"),
            hf_damp: c.f("reverb_damp"), eq_gain: c.f("reverb_eqgain"), eq_freq: c.f("reverb_eqfreq"),
        };
        let positions = [c.i("pos_delay"), c.i("pos_phaser"), c.i("pos_flanger"), c.i("pos_chorus"), c.i("pos_reverb")];
        chain.configure(&delay, &phaser, &flanger, &chorus, &reverb, positions);
        Self { chain }
    }
}
impl Runner for ChainRunner {
    fn set(&mut self, k: &str, v: f32) {
        match k {
            "bpm" => self.chain.set_bpm(v),
            "noteon" => self.chain.note_on(),
            _ => panic!("chain: clé dynamique inconnue {k}"),
        }
    }
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) { self.chain.process(l, r) }
}

fn make_runner(c: &Cfg, rate: f32) -> Box<dyn Runner> {
    match c.kind.as_str() {
        "delay" => Box::new(DelayRunner::new(c, rate)),
        "chorus" => Box::new(ChorusRunner::new(c, rate)),
        "flanger" => Box::new(FlangerRunner::new(c, rate)),
        "phaser" => Box::new(PhaserRunner::new(c, rate)),
        "reverb" => Box::new(ReverbRunner::new(c, rate)),
        "chain" => Box::new(ChainRunner::new(c, rate)),
        other => panic!("genre inconnu: {other}"),
    }
}

fn read_scenarios(path: &Path) -> Result<Vec<Cfg>, Box<dyn Error>> {
    let mut out = Vec::new();
    for line in fs::read_to_string(path)?.lines() {
        if line.is_empty() || line.starts_with('#') { continue; }
        let mut words = line.split_whitespace();
        let (name, kind, stimulus) = (words.next().unwrap(), words.next().unwrap(), words.next().unwrap());
        let mut c = Cfg { name: name.into(), kind: kind.into(), stimulus: stimulus.into(), base: HashMap::new(), events: Vec::new() };
        for item in words {
            let (key, value) = item.split_once('=').ok_or("réglage invalide")?;
            let value: f64 = value.parse()?;
            match key.split_once('@') {
                Some((k, frame)) => c.events.push((frame.parse()?, k.into(), value)),
                None => { c.base.insert(key.into(), value); }
            }
        }
        out.push(c);
    }
    Ok(out)
}

fn read_stimulus(path: &Path) -> Result<Vec<(f32, f32)>, Box<dyn Error>> {
    let mut out = Vec::new();
    for line in fs::read_to_string(path)?.lines().skip(1) {
        let mut cols = line.split(',');
        cols.next();
        out.push((cols.next().ok_or("CSV")?.parse()?, cols.next().ok_or("CSV")?.parse()?));
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 { return Err("Usage: compare_fx_render SCENARIOS STIMULI_DIR OUT_DIR RATE".into()); }
    let (stimuli, output) = (Path::new(&args[2]), Path::new(&args[3]));
    let rate: f32 = args[4].parse()?;
    fs::create_dir_all(output)?;
    let mut cache: HashMap<String, Vec<(f32, f32)>> = HashMap::new();
    for mut scenario in read_scenarios(Path::new(&args[1]))? {
        if !cache.contains_key(&scenario.stimulus) {
            cache.insert(scenario.stimulus.clone(), read_stimulus(&stimuli.join(format!("{}.csv", scenario.stimulus)))?);
        }
        let input = &cache[&scenario.stimulus];
        let mut runner = make_runner(&scenario, rate);
        scenario.events.sort_by_key(|e| e.0);
        let mut next = 0;
        let mut bytes = Vec::with_capacity(input.len() * 16);
        for (frame, &(l, r)) in input.iter().enumerate() {
            while next < scenario.events.len() && scenario.events[next].0 == frame {
                runner.set(&scenario.events[next].1, scenario.events[next].2 as f32);
                next += 1;
            }
            let (ol, or) = runner.process(l, r);
            if !ol.is_finite() || !or.is_finite() { return Err(format!("sortie non finie: {}", scenario.name).into()); }
            bytes.extend_from_slice(&(ol as f64).to_le_bytes());
            bytes.extend_from_slice(&(or as f64).to_le_bytes());
        }
        fs::File::create(output.join(format!("{}.f64", scenario.name)))?.write_all(&bytes)?;
    }
    Ok(())
}
