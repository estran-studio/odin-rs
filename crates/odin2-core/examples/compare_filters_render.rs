//! Banc filtres: rend les mêmes cas que tools/fidelity/filters_modules.cpp.
//! Usage: compare_filters_render CASES STIMULI OUT RATE
use odin2_core::dsp::filters::{
    BiquadAllpass, CombFilter, DiodeFilter, FilterModulation, FormantFilter, Korg35Filter, PresetFilter, SemFilter, Vowel,
};
use odin2_core::dsp::filters::Filter;
use odin2_core::preset::{FilterParams, FilterType};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

struct Case {
    name: String,
    kind: String,
    values: HashMap<String, f64>,
}

impl Case {
    fn get(&self, key: &str, fallback: f64) -> f64 {
        self.values.get(key).copied().unwrap_or(fallback)
    }
    fn f(&self, key: &str, fallback: f64) -> f32 {
        self.get(key, fallback) as f32
    }
}

fn parse_cases(path: &PathBuf) -> Vec<Case> {
    let text = fs::read_to_string(path).expect("CASES illisible");
    let mut cases = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(name), Some(kind)) = (words.next(), words.next()) else { continue };
        let mut values = HashMap::new();
        for token in words {
            let (key, value) = token.split_once('=').expect("jeton invalide");
            values.insert(key.to_string(), value.parse::<f64>().expect("nombre invalide"));
        }
        cases.push(Case { name: name.to_string(), kind: kind.to_string(), values });
    }
    cases
}

fn read_stimulus(path: &PathBuf) -> Vec<(String, f32)> {
    let text = fs::read_to_string(path).expect("stimulus illisible");
    text.lines()
        .skip(1)
        .map(|line| {
            let (frame, value) = line.split_once(',').expect("CSV stimulus incorrect");
            (frame.to_string(), value.parse::<f32>().expect("flottant"))
        })
        .collect()
}

type Process = Box<dyn FnMut(f32) -> f32>;

fn make_module(c: &Case, rate: f32) -> Process {
    let freq = c.f("freq", 1000.0);
    let res = c.f("res", 0.25);
    let od = c.f("od", 0.0);
    match c.kind.as_str() {
        "diode" => {
            let mut f = DiodeFilter::new(rate);
            f.set_cutoff(freq);
            f.set_resonance(res);
            f.set_overdrive(od);
            f.reset();
            Box::new(move |x| f.process(x))
        }
        "korg" => {
            let mut f = Korg35Filter::new(rate);
            if c.get("lp", 1.0) != 0.0 { f.set_lowpass() } else { f.set_highpass() }
            f.set_cutoff(freq);
            f.set_resonance(res);
            f.set_overdrive(od);
            f.reset();
            Box::new(move |x| f.process(x))
        }
        "sem" => {
            let mut f = SemFilter::new(rate);
            f.set_cutoff(freq);
            f.set_resonance(res);
            f.set_transition(c.f("trans", -1.0));
            f.reset();
            Box::new(move |x| f.process(x))
        }
        "formant" => {
            let mut f = FormantFilter::new(rate);
            f.set_vowel_left(Vowel::from(c.get("vl", 0.0) as u8));
            f.set_vowel_right(Vowel::from(c.get("vr", 1.0) as u8));
            f.set_transition(c.f("trans", 0.0));
            f.reset();
            Box::new(move |x| f.process(x))
        }
        "comb" => {
            let mut f = CombFilter::new(rate);
            f.set_positive(c.get("pos", 1.0) != 0.0);
            f.set_resonance(res);
            f.set_frequency(freq);
            if c.get("noreset", 0.0) == 0.0 {
                f.reset();
            }
            Box::new(move |x| {
                f.set_frequency(freq);
                f.process(x)
            })
        }
        "allpass" => {
            let mut f = BiquadAllpass::new(rate);
            f.set_radius(c.f("radius", 1.32));
            f.set_frequency(freq);
            f.reset();
            Box::new(move |x| f.process(x))
        }
        other => panic!("genre module inconnu côté Rust: {other}"),
    }
}

fn make_preset(c: &Case, rate: f32) -> Process {
    let filter_type = FilterType::from(c.get("type", 2.0) as i32);
    let freq = c.f("freq", 1000.0);
    let gain_db = c.f("gain", 0.0);
    let params = FilterParams {
        filter_type,
        frequency: freq,
        resonance: c.f("res", 0.25),
        saturation: c.f("sat", 0.0),
        gain: gain_db,
        env_amount: c.f("envamt", 0.0),
        vel_amount: c.f("velamt", 0.0),
        kbd_follow: c.f("kbd", 0.0),
        sem_transition: c.f("trans", -1.0),
        formant_transition: c.f("ftrans", 0.0),
        ring_mod_amount: c.f("ring", 1.0),
        comb_polarity: c.get("pol", 0.0) != 0.0,
        vowel_left: c.get("vl", 0.0) as i32,
        vowel_right: c.get("vr", 1.0) as i32,
        ..FilterParams::default()
    };
    let modulation = FilterModulation {
        frequency: c.f("mfreq", 0.0),
        resonance: c.f("mres", 0.0),
        gain: c.f("mgain", 0.0),
        saturation: c.f("msat", 0.0),
        envelope: c.f("menv", 0.0),
        velocity: c.f("mvel", 0.0),
        keyboard: c.f("mkbd", 0.0),
        sem_transition: c.f("mtrans", 0.0),
        formant_transition: c.f("mtrans", 0.0),
        ring_amount: c.f("mring", 0.0),
    };
    let mut f = PresetFilter::new(rate);
    f.configure(&params);
    f.set_midi_unshifted(c.get("note", 0.0) as u8, c.get("velocity", 0.0) as u8);
    f.set_env_value(c.f("env", 0.0));
    f.set_modulation(&modulation);
    let gain_linear = if gain_db <= -59.99 { 0.0 } else { 10f32.powf(gain_db * 0.05) };
    f.set_smoothing_state(freq, gain_linear);
    f.reset();
    Box::new(move |x| f.process(x))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 5, "Usage: compare_filters_render CASES STIMULI OUT RATE");
    let cases = parse_cases(&PathBuf::from(&args[1]));
    let stimuli = PathBuf::from(&args[2]);
    let out = PathBuf::from(&args[3]);
    let rate: f32 = args[4].parse().expect("RATE");
    fs::create_dir_all(&out).unwrap();
    for c in &cases {
        for scenario in ["impulse", "sine", "sweep"] {
            let input = read_stimulus(&stimuli.join(format!("{scenario}.csv")));
            let mut process = if c.kind == "preset" { make_preset(c, rate) } else { make_module(c, rate) };
            let mut text = String::from("frame,input,output\n");
            for (frame, x) in input {
                let y = process(x);
                assert!(y.is_finite(), "sortie Rust non finie: {} {scenario}", c.name);
                writeln!(text, "{frame},{x},{y}").unwrap();
            }
            fs::write(out.join(format!("{}_{scenario}.csv", c.name)), text).unwrap();
        }
    }
}
