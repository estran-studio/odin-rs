//! Banc oscillateurs/LFO côté Rust: même fichier de scénarios que tools/fidelity/osc_modules.cpp.
//! Usage: compare_osc_render SPEC STIMULI OUT RATE
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use odin2_core::mod_matrix::{GlobalSources, ModMatrix};
use odin2_core::voice::{Voice, VoiceConfig};
use odin2_core::dsp::oscillators::preset_oscillator::{map_chip, map_vector, MORPH_TABLES};
use odin2_core::dsp::oscillators::{DrawTableBank, Lfo, OscillatorModulation, PresetOscillator};
use odin2_core::preset::{AnalogWaveform, EnvelopeParams, FilterParams, FilterType, LfoParams, LfoWaveform, OscillatorParams, OscillatorType};
use odin2_core::wavetables::{get_subtable, lfo_data::LFO_DATA};

struct Spec {
    name: String,
    kind: String,
    stim: String,
    kv: BTreeMap<String, f64>,
}
impl Spec {
    fn get(&self, key: &str, default: f64) -> f64 { *self.kv.get(key).unwrap_or(&default) }
    fn geti(&self, key: &str, default: i32) -> i32 { self.get(key, default as f64) as i32 }
    fn getf(&self, key: &str, default: f32) -> f32 { self.get(key, default as f64) as f32 }
}

fn read_specs(path: &Path) -> Result<(Vec<Spec>, u32), Box<dyn Error>> {
    let mut specs = Vec::new();
    let mut seed = 1u32;
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if let Some(rest) = line.strip_prefix("# drawseed ") { seed = rest.trim().parse()?; continue; }
        if line.is_empty() || line.starts_with('#') { continue; }
        let mut words = line.split_whitespace();
        let name = words.next().ok_or("nom")?.to_string();
        let kind = words.next().ok_or("type")?.to_string();
        let stim = words.next().ok_or("stimulus")?.to_string();
        let mut kv = BTreeMap::new();
        for word in words {
            let (key, value) = word.split_once('=').ok_or("clé=valeur")?;
            kv.insert(key.to_string(), value.parse()?);
        }
        specs.push(Spec { name, kind, stim, kv });
    }
    Ok((specs, seed))
}

/// Même générateur que osc_modules.cpp::draw_values.
fn draw_values(seed: u32, count: usize, amplitude: f32) -> Vec<f32> {
    let mut x = seed.wrapping_mul(2654435761).wrapping_add(12345);
    (0..count).map(|_| {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        let unit = ((x >> 8) & 0xffff) as f32 / 32768.0 - 1.0;
        unit * amplitude
    }).collect()
}

fn draw_params(seed: u32) -> [OscillatorParams; 3] {
    core::array::from_fn(|osc| {
        let mut params = OscillatorParams::default();
        let wave = draw_values(seed * 16 + osc as u32 * 3, 193, 0.9);
        let chip = draw_values(seed * 16 + osc as u32 * 3 + 1, 32, 0.9);
        let spec = draw_values(seed * 16 + osc as u32 * 3 + 2, 49, 1.0);
        params.wavedraw.copy_from_slice(&wave);
        params.chipdraw.copy_from_slice(&chip);
        for (target, value) in params.specdraw.iter_mut().zip(spec) { *target = (value + 1.0) * 0.5; }
        params
    })
}

fn dump_static(out: &Path, bank: &DrawTableBank) -> Result<(), Box<dyn Error>> {
    let mut data = BufWriter::new(File::create(out.join("tables.f32"))?);
    for wt in 0..160 { for sub in 0..33 { for value in get_subtable(wt, sub) { data.write_all(&value.to_le_bytes())?; } } }
    let mut lfo = BufWriter::new(File::create(out.join("lfo.f32"))?);
    for table in LFO_DATA.iter() { for value in table { lfo.write_all(&value.to_le_bytes())?; } }
    let mut mappings = BufWriter::new(File::create(out.join("mappings.txt"))?);
    for code in 0..1000 {
        let vector = map_vector(code);
        let fm = if code == 1 { 0 } else { map_vector(code) };
        writeln!(mappings, "{code} {vector} {fm} {}", map_chip(code))?;
    }
    let mut morph = BufWriter::new(File::create(out.join("morph.txt"))?);
    for (wt, tables) in MORPH_TABLES.iter().enumerate() {
        writeln!(morph, "{wt} {} {} {} {}", tables[0], tables[1], tables[2], tables[3])?;
    }
    for (kind, name) in ["wave", "chip", "spec"].iter().enumerate() {
        for osc in 0..3 {
            let mut file = BufWriter::new(File::create(out.join(format!("draw_{name}{osc}.f32")))?);
            for sub in 0..33 {
                for value in bank.subtable(kind * 3 + osc, sub) { file.write_all(&value.to_le_bytes())?; }
            }
        }
    }
    Ok(())
}

fn analog_wave(index: i32) -> AnalogWaveform { AnalogWaveform::from(index) }

fn build_params(spec: &Spec) -> OscillatorParams {
    let mut p = OscillatorParams {
        octave: spec.geti("oct", 0),
        semitones: spec.geti("semi", 0),
        fine: spec.getf("fine", 0.0),
        drift: 0.0,
        sync: spec.geti("sync", 0) != 0,
        reset: false,
        ..OscillatorParams::default()
    };
    match spec.kind.as_str() {
        "analog" => {
            p.osc_type = OscillatorType::Analog;
            p.analog_wave = analog_wave(spec.geti("wave", 0));
            p.pulsewidth = spec.getf("pw", 0.5);
        }
        "wt2d" => {
            p.osc_type = OscillatorType::Wavetable;
            p.wavetable = spec.geti("wt", 1);
            p.position = spec.getf("pos", 0.0);
            p.pos_mod = spec.getf("pamt", 0.0);
        }
        "multi" => {
            p.osc_type = OscillatorType::Multi;
            p.wavetable = spec.geti("wt", 1);
            p.multi_position = spec.getf("pos", 0.0);
            p.detune = spec.getf("detune", 0.2);
            p.spread = spec.getf("spread", 0.0);
        }
        "vector" => {
            p.osc_type = OscillatorType::Vector;
            p.vec_a = spec.geti("a", 101);
            p.vec_b = spec.geti("b", 102);
            p.vec_c = spec.geti("c", 103);
            p.vec_d = spec.geti("d", 104);
            p.vec_x = spec.getf("x", 0.0);
            p.vec_y = spec.getf("y", 0.0);
        }
        "chiptune" => {
            p.osc_type = OscillatorType::Chiptune;
            p.chipwave = spec.geti("wave", 1);
            p.arp_on = spec.geti("arp", 0) != 0;
            p.arp_speed = spec.getf("aspeed", 20.0);
            p.step_1 = spec.geti("s1", 0);
            p.step_2 = spec.geti("s2", 12);
            p.step_3 = spec.geti("s3", 7);
            p.step_3_on = spec.geti("s3on", 0) != 0;
            p.chipnoise = spec.geti("noise", 0) != 0;
        }
        "fm" | "pm" => {
            p.osc_type = if spec.kind == "fm" { OscillatorType::FM } else { OscillatorType::PM };
            p.carrier_wave = spec.geti("cwave", 1);
            p.modulator_wave = spec.geti("mwave", 1);
            p.fm = spec.getf("amount", 0.0);
            p.carrier_ratio = spec.geti("cr", 1);
            p.modulator_ratio = spec.geti("mr", 1);
            p.exp_fm = spec.geti("expfm", 0) != 0;
        }
        "noise" => {
            p.osc_type = OscillatorType::Noise;
            p.lp = spec.getf("lp", 18000.0);
            p.hp = spec.getf("hp", 80.0);
        }
        "wt1d" => {
            p.osc_type = match spec.geti("kind", 0) {
                0 => OscillatorType::Wavedraw,
                1 => OscillatorType::Chipdraw,
                _ => OscillatorType::Specdraw,
            };
        }
        other => panic!("type inconnu {other}"),
    }
    p
}

fn build_modulation(spec: &Spec) -> OscillatorModulation {
    OscillatorModulation {
        exponential_fm: spec.getf("exp", 0.0),
        linear_fm: spec.getf("lin", 0.0),
        pulse_width: spec.getf("pwm", 0.0),
        position: spec.getf("pmod", 0.0),
        vector_x: spec.getf("xmod", 0.0),
        vector_y: spec.getf("ymod", 0.0),
        multi_detune: spec.getf("dmod", 0.0),
        spread: spec.getf("smod", 0.0),
        arp_speed: spec.getf("amod", 0.0),
        noise_hp: spec.getf("hmod", 0.0),
        noise_lp: spec.getf("lmod", 0.0),
        fm_amount: spec.getf("amod", 0.0),
        carrier_ratio: spec.getf("crmod", 0.0),
        fm_ratio: spec.getf("mrmod", 0.0),
        ..Default::default()
    }
}

fn run(spec: &Spec, bank: &Arc<DrawTableBank>, rate: u32, stimuli: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(File::create(out.join(format!("{}.csv", spec.name)))?);
    writeln!(writer, "frame,input,output")?;
    let reader = BufReader::new(File::open(stimuli.join(&spec.stim))?);
    let mut step: Box<dyn FnMut(f32) -> f32> = if spec.kind == "voice" {
        // Voice complète (glide, détune d'unisson, pitch bend): sinus analogique, aucun filtre, sans
        // modulation. Seuls les passages par zéro sont comparés (le gain de voix diffère).
        let mut config = VoiceConfig::default();
        config.oscillators[0] = OscillatorParams { osc_type: OscillatorType::Analog,
            analog_wave: analog_wave(spec.geti("wave", 3)), ..OscillatorParams::default() };
        config.oscillators[1].osc_type = OscillatorType::None;
        config.oscillators[2].osc_type = OscillatorType::None;
        config.filters[0] = FilterParams { filter_type: FilterType::None, osc1_input: true, osc2_input: false,
            osc3_input: false, to_amp: true, ..FilterParams::default() };
        config.filters[1] = FilterParams { filter_type: FilterType::None, osc1_input: false, osc2_input: false,
            osc3_input: false, fil1_input: false, to_amp: false, ..FilterParams::default() };
        config.envelopes[0] = EnvelopeParams { attack: 0.001, decay: 1.0, sustain: 1.0, release: 0.1, loop_env: false };
        config.glide = spec.getf("glide", 0.0);
        config.pitch_bend_range = 12.0;
        let mut voice = Voice::new(rate as f32);
        voice.configure(&config);
        let note = spec.geti("note", 69) as u8;
        let last = spec.geti("last", note as i32) as u8;
        voice.start(note, 100, Some(last), 0.0, spec.getf("unison", 0.0), 1.0, false);
        let matrix = ModMatrix::new();
        let globals = GlobalSources { pitchwheel: spec.getf("bend", 0.0) / 12.0, ..GlobalSources::default() };
        Box::new(move |_| voice.process_with_modulation(&matrix, &globals).0)
    } else if spec.kind == "lfo" {
        let mut lfo = Lfo::new(rate as f32);
        lfo.set_draw_tables(Arc::clone(bank));
        lfo.configure(&LfoParams {
            waveform: LfoWaveform::from(spec.geti("wave", 0)),
            frequency: spec.getf("freq", 1.0),
            reset: false,
            sync: spec.geti("sync", 0) != 0,
            sync_numerator: spec.geti("num", 2),
            sync_denominator: spec.geti("den", 5),
        });
        lfo.set_tempo(spec.getf("bpm", 120.0));
        lfo.set_modulation(spec.getf("mod", 0.0));
        lfo.reset();
        Box::new(move |_| lfo.process())
    } else {
        let params = build_params(spec);
        let modulation = build_modulation(spec);
        let mut osc = PresetOscillator::new(rate as f32);
        osc.set_draw_tables(Arc::clone(bank));
        osc.set_oscillator_index(spec.geti("idx", 0) as usize);
        osc.set_seed(spec.get("seed", 1.0) as u32 + 1);
        osc.configure(&params);
        osc.set_position_source(spec.getf("psrc", 0.0));
        osc.set_modulation(modulation);
        osc.reset();
        if spec.geti("randphase", 0) != 0 { osc.randomize_phase(); }
        osc.voice_start(false);
        let sync = params.sync;
        let mut master = if sync {
            let mut master_params = OscillatorParams { osc_type: OscillatorType::Analog, analog_wave: AnalogWaveform::Saw,
                drift: 0.0, ..OscillatorParams::default() };
            master_params.semitones = spec.geti("msemi", 0);
            let mut master = PresetOscillator::new(rate as f32);
            master.configure(&master_params);
            master.reset();
            Some(master)
        } else { None };
        Box::new(move |frequency| {
            if let Some(master) = master.as_mut() {
                master.set_frequency(frequency);
                master.process();
                osc.set_sync_input(master.did_wrap());
                osc.set_sync_position(master.reset_position());
            }
            osc.set_frequency(frequency);
            osc.process()
        })
    };
    for line in reader.lines().skip(1) {
        let line = line?;
        let (frame, input) = line.split_once(',').ok_or("CSV stimulus incorrect")?;
        let frequency: f32 = input.parse()?;
        let value = step(frequency);
        if !value.is_finite() { return Err(format!("non fini: {}", spec.name).into()); }
        writeln!(writer, "{frame},{frequency:.9e},{value:.9e}")?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 { return Err("Usage: compare_osc_render SPEC STIMULI OUT RATE".into()); }
    let (specs, seed) = read_specs(Path::new(&args[1]))?;
    let (stimuli, out, rate) = (Path::new(&args[2]), Path::new(&args[3]), args[4].parse::<u32>()?);
    fs::create_dir_all(out)?;
    let bank = Arc::new(DrawTableBank::new(&draw_params(seed)));
    dump_static(out, &bank)?;
    for spec in &specs { run(spec, &bank, rate, stimuli, out)?; }
    Ok(())
}
