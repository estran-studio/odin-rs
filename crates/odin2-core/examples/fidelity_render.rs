//! Rendu hors temps réel pour scripts/fidelity.py; aucun périphérique audio.
use odin2_core::dsp::effects::OversamplingDistortion;
use odin2_core::dsp::envelopes::{Adsr, Envelope};
use odin2_core::dsp::filters::biquad::BiquadFilter;
use odin2_core::dsp::filters::{ladder::LadderFilter, va_one_pole::VAOnePoleFilter, Filter, LadderFilterType};
use odin2_core::dsp::oscillators::PresetOscillator;
use odin2_core::engine::{OdinEngine, SynthEngine};
use odin2_core::preset::{
    AnalogWaveform, DistortionAlgo, FilterType, LfoWaveform, OdinPreset, OscillatorParams, OscillatorType, PlayMode,
};
use odin2_core::{EngineConfig, ParameterId};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("preset") if args.len() >= 7 => render_preset(
            Path::new(&args[2]), Path::new(&args[3]), args[4].parse()?,
            &args[5], args[6].parse()?, &parse_overrides(&args[7..])?,
        ),
        Some("modules") if args.len() == 5 => {
            render_modules(Path::new(&args[2]), Path::new(&args[3]), args[4].parse()?)
        }
        _ => Err("Usage: fidelity_render preset PATCH OUT.wav RATE note|pattern SECONDS [--set nom=valeur | --set noeud/propriété=valeur]... | modules STIMULI_DIR OUT_DIR RATE".into()),
    }
}

fn render_preset(
    preset_path: &Path, output: &Path, rate: u32, scenario: &str, seconds: f64,
    overrides: &[(String, f64)],
) -> Result<(), Box<dyn Error>> {
    if rate < 8000 || !seconds.is_finite() || seconds < 3.0 {
        return Err("RATE >= 8000 et SECONDS >= 3 requis".into());
    }
    let events: &[(f64, bool, u8)] = match scenario {
        "note" => &[(0.25, true, 48), (2.25, false, 48)],
        "pattern" => &[
            (0.25, true, 48), (0.75, true, 55), (1.25, false, 48),
            (1.25, true, 60), (1.75, false, 55), (2.25, false, 60),
        ],
        _ => return Err("Scénario attendu: note ou pattern".into()),
    };
    let preset = OdinPreset::load(preset_path)?;
    // Certaines configurations contiennent de gros tableaux de voix: allocation hors audio.
    let mut engine = Box::new(OdinEngine::new(rate as f32));
    let mut config = EngineConfig::from_preset(&preset);
    for (name, value) in overrides { apply_override(&mut config, name, *value)?; }
    engine.load_config(config)?;
    engine.set_tempo(120.0);
    let spec = hound::WavSpec {
        channels: 2, sample_rate: rate, bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(output, spec)?;
    let frames = (seconds * rate as f64).round() as usize;
    let mut cursor = 0;
    let mut event_index = 0;
    let mut buffer = [0.0f32; 512];
    while cursor < frames {
        while event_index < events.len()
            && (events[event_index].0 * rate as f64).round() as usize == cursor
        {
            let (_, on, note) = events[event_index];
            if on { engine.note_on(note, 100); } else { engine.note_off(note); }
            event_index += 1;
        }
        let next_event = events.get(event_index)
            .map(|event| (event.0 * rate as f64).round() as usize).unwrap_or(frames);
        let count = 256.min(frames - cursor).min(next_event - cursor);
        buffer[..count * 2].fill(0.0);
        engine.process(&mut buffer[..count * 2], 2);
        for sample in &buffer[..count * 2] {
            if !sample.is_finite() { return Err("Échantillon Rust non fini".into()); }
            writer.write_sample(*sample)?;
        }
        cursor += count;
    }
    writer.finalize()?;
    Ok(())
}

/// `--set nom=valeur` (paramètre du catalogue, unités physiques) ou
/// `--set noeud/propriété=valeur` (propriété non audio du ValueTree), répétable.
fn parse_overrides(args: &[String]) -> Result<Vec<(String, f64)>, Box<dyn Error>> {
    let mut overrides = Vec::new();
    let mut iterator = args.iter();
    while let Some(flag) = iterator.next() {
        let assignment = match (flag.as_str(), iterator.next()) {
            ("--set", Some(assignment)) => assignment,
            _ => return Err("Arguments supplémentaires attendus: --set nom=valeur | noeud/propriété=valeur".into()),
        };
        let (name, text) = assignment.split_once('=')
            .filter(|(name, _)| !name.is_empty())
            .ok_or_else(|| format!("--set attend nom=valeur: {assignment}"))?;
        let value: f64 = text.trim().parse()
            .map_err(|_| format!("--set {name}: valeur numérique invalide '{text}'"))?;
        if !value.is_finite() { return Err(format!("--set {name}: valeur non finie").into()); }
        overrides.push((name.to_string(), value));
    }
    Ok(overrides)
}

fn apply_override(config: &mut EngineConfig, name: &str, value: f64) -> Result<(), Box<dyn Error>> {
    let Some((node, property)) = name.split_once('/') else {
        let id = ParameterId::from_name(name).map_err(|_| format!("--set {name}: paramètre audio inconnu"))?;
        return Ok(id.set(config, value as f32)?);
    };
    let integer = || -> Result<i32, String> {
        if value.fract() != 0.0 || value.abs() > i32::MAX as f64 {
            return Err(format!("--set {name}: entier attendu, reçu {value}"));
        }
        Ok(value as i32)
    };
    let boolean = value != 0.0;
    let unknown = || format!("--set {name}: propriété non audio non prise en charge");
    // Préfixe + indice 1-based + "_" + suffixe, ex. osc2_type, fil3_vowel_left.
    let indexed = |prefix: &str, count: usize| -> Option<(usize, String)> {
        let rest = property.strip_prefix(prefix)?;
        let digit = rest.chars().next()?.to_digit(10)? as usize;
        if digit < 1 || digit > count { return None; }
        Some((digit - 1, rest[1..].strip_prefix('_')?.to_string()))
    };
    match node {
        "mod" => {
            let (field, row) = property.rsplit_once("_row_").ok_or_else(unknown)?;
            let row: usize = row.parse().map_err(|_| unknown())?;
            let row = config.mod_matrix.get_mut(row).ok_or_else(|| format!("--set {name}: ligne hors 0..9"))?;
            match field {
                "source" => row.source = integer()?,
                "dest_1" => row.dest1 = integer()?,
                "dest_2" => row.dest2 = integer()?,
                "scale" => row.scale = integer()?,
                "amount_0" => row.dest1_amount = value as f32,
                "amount_1" => row.dest2_amount = value as f32,
                "amount_2" => row.scale_amount = value as f32,
                _ => return Err(unknown().into()),
            }
        }
        "misc" => {
            if let Some((index, suffix)) = indexed("fil", 3) {
                let filter = [&mut config.filter1, &mut config.filter2, &mut config.filter3].into_iter().nth(index).unwrap();
                match suffix.as_str() {
                    "type" => filter.filter_type = FilterType::from(integer()?),
                    "comb_polarity" => filter.comb_polarity = boolean,
                    "vowel_left" => filter.vowel_left = integer()?,
                    "vowel_right" => filter.vowel_right = integer()?,
                    _ => return Err(unknown().into()),
                }
                return Ok(());
            }
            match property {
                "dist_on" => config.distortion.on = boolean,
                "dist_algo" => config.distortion.algorithm = DistortionAlgo::from(integer()?),
                "unison_voices" => config.unison_voices = integer()?,
                "legato" => config.play_mode = PlayMode::from(integer()?),
                "pitchbend_amount" => config.pitchbend_amount = integer()?,
                "BPM" => config.bpm = value as f32,
                _ => return Err(unknown().into()),
            }
        }
        "osc" => {
            let (index, suffix) = indexed("osc", 3).ok_or_else(unknown)?;
            let osc = [&mut config.osc1, &mut config.osc2, &mut config.osc3].into_iter().nth(index).unwrap();
            match suffix.as_str() {
                "type" => osc.osc_type = OscillatorType::from(integer()?),
                "analog_wave" => osc.analog_wave = AnalogWaveform::from(integer()?),
                "wavetable" => osc.wavetable = integer()?,
                "vec_a" => osc.vec_a = integer()?,
                "vec_b" => osc.vec_b = integer()?,
                "vec_c" => osc.vec_c = integer()?,
                "vec_d" => osc.vec_d = integer()?,
                "chipwave" => osc.chipwave = integer()?,
                "modulator_wave" => osc.modulator_wave = integer()?,
                "carrier_wave" => osc.carrier_wave = integer()?,
                "mod_source" => osc.mod_source = integer()?,
                "carrier_ratio" => osc.carrier_ratio = integer()?,
                "modulator_ratio" => osc.modulator_ratio = integer()?,
                _ => return Err(unknown().into()),
            }
        }
        "lfo" => {
            let (index, suffix) = indexed("lfo", 4).ok_or_else(unknown)?;
            let lfo = [&mut config.lfo1, &mut config.lfo2, &mut config.lfo3, &mut config.lfo4].into_iter().nth(index).unwrap();
            match suffix.as_str() {
                "wave" => lfo.waveform = LfoWaveform::from(integer()?),
                "sync" => lfo.sync = boolean,
                "synctime_numerator" => lfo.sync_numerator = integer()?,
                "synctime_denominator" => lfo.sync_denominator = integer()?,
                _ => return Err(unknown().into()),
            }
        }
        "fx" => {
            let (effect, suffix) = property.split_once('_').ok_or_else(unknown)?;
            let position = ["delay", "phaser", "flanger", "chorus", "reverb"].iter().position(|n| *n == effect)
                .ok_or_else(unknown)?;
            if suffix == "position" { config.fx_positions[position] = integer()?; return Ok(()); }
            macro_rules! sync_fields {
                ($params:expr) => {
                    match suffix {
                        "sync" => $params.sync = boolean,
                        "synctime_numerator" => $params.sync_numerator = integer()?,
                        "synctime_denominator" => $params.sync_denominator = integer()?,
                        _ => return Err(unknown().into()),
                    }
                };
            }
            match effect {
                "delay" => sync_fields!(config.delay),
                "phaser" => sync_fields!(config.phaser),
                "flanger" => sync_fields!(config.flanger),
                "chorus" => sync_fields!(config.chorus),
                _ => return Err(unknown().into()),
            }
        }
        _ => return Err(format!("--set {name}: noeud '{node}' non pris en charge (mod, misc, osc, lfo, fx)").into()),
    }
    Ok(())
}

fn render_modules(stimuli: &Path, output: &Path, rate: u32) -> Result<(), Box<dyn Error>> {
    if rate < 8000 { return Err("RATE >= 8000 requis".into()); }
    std::fs::create_dir_all(output)?;
    let capture = |stimulus: &str, name: &str, process: &mut dyn FnMut(f32) -> f32| -> Result<(), Box<dyn Error>> {
        let reader = BufReader::new(File::open(stimuli.join(stimulus))?);
        let mut writer = BufWriter::new(File::create(output.join(name))?);
        writeln!(writer, "frame,input,output")?;
        for line in reader.lines().skip(1) {
            let line = line?;
            let (frame, input) = line.split_once(',').ok_or("CSV stimulus incorrect")?;
            let input: f32 = input.parse()?;
            let value = process(input);
            if !value.is_finite() { return Err(format!("Module Rust non fini: {name}").into()); }
            writeln!(writer, "{frame},{input:.9e},{value:.9e}")?;
        }
        Ok(())
    };
    for scenario in ["impulse", "sine", "sweep"] {
        let stimulus = format!("{scenario}.csv");
        let mut filter = BiquadFilter::new(rate as f32);
        filter.a1 = -0.5;
        filter.a2 = 0.125;
        filter.b0 = 0.125;
        filter.b1 = 0.25;
        filter.b2 = 0.125;
        capture(&stimulus, &format!("biquad_{scenario}.csv"), &mut |x| filter.process(x))?;
        for (name, kind) in [("lp24", LadderFilterType::LP4), ("lp12", LadderFilterType::LP2),
            ("bp24", LadderFilterType::BP4), ("bp12", LadderFilterType::BP2),
            ("hp24", LadderFilterType::HP4), ("hp12", LadderFilterType::HP2)] {
            let mut filter = LadderFilter::new(rate as f32);
            filter.set_filter_type(kind);
            filter.set_cutoff(1000.0);
            filter.set_resonance(0.25);
            filter.reset();
            capture(&stimulus, &format!("ladder_{name}_{scenario}.csv"), &mut |x| filter.process(x))?;
        }
        for (name, low) in [("lp", true), ("hp", false)] {
            let mut filter = VAOnePoleFilter::new(rate as f32);
            if low { filter.set_lowpass() } else { filter.set_highpass() }
            filter.update(1000.0);
            filter.reset();
            capture(&stimulus, &format!("onepole_{name}_{scenario}.csv"), &mut |x| filter.process(x))?;
        }
        for (name, algorithm) in [("clamp", DistortionAlgo::Clamp), ("fold", DistortionAlgo::Fold),
            ("zero", DistortionAlgo::Zero), ("sine", DistortionAlgo::Sine), ("cube", DistortionAlgo::Cube)] {
            let mut distortion = OversamplingDistortion::new();
            distortion.set_algorithm(algorithm);
            distortion.set_threshold(0.5);
            distortion.set_dry_wet(0.75);
            distortion.reset();
            capture(&stimulus, &format!("distortion_{name}_{scenario}.csv"), &mut |x| distortion.process(x))?;
        }
    }
    for (name, waveform) in [("saw", AnalogWaveform::Saw), ("pulse", AnalogWaveform::Pulse),
        ("triangle", AnalogWaveform::Triangle), ("sine", AnalogWaveform::Sine)] {
        for scenario in ["tone", "sweep"] {
            let params = OscillatorParams {
                osc_type: OscillatorType::Analog,
                analog_wave: waveform,
                octave: 0,
                semitones: 0,
                fine: 0.0,
                drift: 0.0,
                pulsewidth: 0.5,
                sync: false,
                ..Default::default()
            };
            let mut oscillator = PresetOscillator::new(rate as f32);
            oscillator.configure(&params);
            oscillator.reset();
            capture(&format!("frequency_{scenario}.csv"), &format!("analog_{name}_{scenario}.csv"), &mut |frequency| {
                oscillator.set_frequency(frequency);
                oscillator.process()
            })?;
        }
    }
    let mut envelope = Adsr::new(rate as f32);
    envelope.set_attack(0.01);
    envelope.set_decay(0.1);
    envelope.set_sustain(0.4);
    envelope.set_release(0.2);
    envelope.trigger();
    let mut writer = BufWriter::new(File::create(output.join("adsr_gate.csv"))?);
    writeln!(writer, "frame,input,output")?;
    for frame in 0..rate as usize * 2 {
        if frame == rate as usize * 3 / 4 { envelope.release(); }
        writeln!(writer, "{frame},{},{:.9e}", u8::from(frame < rate as usize * 3 / 4), envelope.process())?;
    }
    Ok(())
}
