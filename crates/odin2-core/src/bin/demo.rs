//! Odin 2 Demo - WAV Generator
//!
//! Generates WAV files with different synthesizer presets and melodies.
//!
//! Usage:
//!   cargo run --bin odin2-demo --features std -- [OPTIONS]
//!
//! Options:
//!   --list              List all available presets
//!   --preset NAME       Use a specific preset (default: all)
//!   --melody NAME       Use a specific melody (default: arpeggio)
//!   --output DIR        Output directory (default: ../../samples/demos)
//!   --play              Play audio after generation (macOS only)
//!   --factory NAME      Load and render a factory preset
//!   --list-factory      List all factory presets

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use hound::{WavSpec, WavWriter};

use odin2_core::dsp::effects::{Chorus, Delay, ZitaReverb};
use odin2_core::dsp::envelopes::{Adsr, Envelope};
use odin2_core::dsp::filters::{Filter, LadderFilter, LadderFilterType};
use odin2_core::dsp::oscillators::{
    AnalogOscillator, MultiOscillator, Oscillator, WavetableOsc2D, WavetableOscillator, Waveform,
};
use odin2_core::preset::OdinPreset;
use odin2_core::{OdinEngine, SynthEngine};

const SAMPLE_RATE: u32 = 44100;

/// Melody definition
struct Melody {
    name: &'static str,
    notes: &'static [(u8, f32, f32)], // (midi_note, start_time, duration)
    duration: f32,
}

/// Available melodies
const MELODIES: &[Melody] = &[
    Melody {
        name: "arpeggio",
        notes: &[
            (60, 0.0, 0.2),
            (64, 0.25, 0.2),
            (67, 0.5, 0.2),
            (72, 0.75, 0.2),
            (67, 1.0, 0.2),
            (64, 1.25, 0.2),
            (60, 1.5, 0.2),
            (64, 1.75, 0.2),
            (67, 2.0, 0.2),
            (72, 2.25, 0.2),
            (76, 2.5, 0.4),
            (72, 3.0, 0.8),
        ],
        duration: 4.5,
    },
    Melody {
        name: "chord_progression",
        notes: &[
            // C major
            (48, 0.0, 1.8),
            (60, 0.0, 1.8),
            (64, 0.0, 1.8),
            (67, 0.0, 1.8),
            // F major
            (53, 2.0, 1.8),
            (60, 2.0, 1.8),
            (65, 2.0, 1.8),
            (69, 2.0, 1.8),
            // G major
            (55, 4.0, 1.8),
            (59, 4.0, 1.8),
            (62, 4.0, 1.8),
            (67, 4.0, 1.8),
            // C major
            (48, 6.0, 1.8),
            (60, 6.0, 1.8),
            (64, 6.0, 1.8),
            (67, 6.0, 1.8),
        ],
        duration: 8.0,
    },
    Melody {
        name: "bass_line",
        notes: &[
            (36, 0.0, 0.4),
            (36, 0.5, 0.2),
            (38, 0.75, 0.2),
            (40, 1.0, 0.4),
            (40, 1.5, 0.2),
            (43, 1.75, 0.2),
            (36, 2.0, 0.4),
            (36, 2.5, 0.2),
            (38, 2.75, 0.2),
            (40, 3.0, 0.4),
            (43, 3.5, 0.4),
        ],
        duration: 4.5,
    },
    Melody {
        name: "lead_melody",
        notes: &[
            (72, 0.0, 0.3),
            (74, 0.5, 0.3),
            (76, 1.0, 0.6),
            (74, 1.75, 0.2),
            (72, 2.0, 0.8),
            (69, 3.0, 0.3),
            (67, 3.5, 0.3),
            (69, 4.0, 0.6),
            (72, 4.75, 0.2),
            (74, 5.0, 1.0),
        ],
        duration: 6.5,
    },
];

/// Synthesizer preset
struct Preset {
    name: &'static str,
    description: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "analog_saw",
        description: "Classic analog sawtooth",
    },
    Preset {
        name: "analog_square",
        description: "Hollow square wave",
    },
    Preset {
        name: "supersaw",
        description: "Detuned unison saw (Trance lead)",
    },
    Preset {
        name: "wavetable_pad",
        description: "Evolving wavetable pad",
    },
    Preset {
        name: "wavetable2d_morph",
        description: "2D wavetable with position sweep",
    },
    Preset {
        name: "filtered_bass",
        description: "Saw bass with filter envelope",
    },
    Preset {
        name: "pluck",
        description: "Short plucky sound",
    },
    Preset {
        name: "pad_reverb",
        description: "Lush pad with reverb",
    },
    Preset {
        name: "delay_lead",
        description: "Lead with ping-pong delay",
    },
    Preset {
        name: "chorus_strings",
        description: "String-like with chorus",
    },
];

fn midi_to_freq(note: u8) -> f32 {
    440.0 * libm::powf(2.0, (note as f32 - 69.0) / 12.0)
}

fn write_stereo_wav(path: &str, samples: &[f32]) {
    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec).expect("Failed to create WAV file");

    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(sample_i16).unwrap();
    }

    writer.finalize().unwrap();
}

/// Generate audio for a preset and melody
fn generate_preset(preset_name: &str, melody: &Melody) -> Vec<f32> {
    let num_samples = (SAMPLE_RATE as f32 * melody.duration) as usize;
    let mut samples = vec![0.0f32; num_samples * 2];

    match preset_name {
        "analog_saw" => generate_analog_saw(melody, &mut samples),
        "analog_square" => generate_analog_square(melody, &mut samples),
        "supersaw" => generate_supersaw(melody, &mut samples),
        "wavetable_pad" => generate_wavetable_pad(melody, &mut samples),
        "wavetable2d_morph" => generate_wavetable2d(melody, &mut samples),
        "filtered_bass" => generate_filtered_bass(melody, &mut samples),
        "pluck" => generate_pluck(melody, &mut samples),
        "pad_reverb" => generate_pad_reverb(melody, &mut samples),
        "delay_lead" => generate_delay_lead(melody, &mut samples),
        "chorus_strings" => generate_chorus_strings(melody, &mut samples),
        _ => eprintln!("Unknown preset: {}", preset_name),
    }

    samples
}

fn generate_analog_saw(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = AnalogOscillator::new(sr);
        osc.set_waveform(Waveform::Saw);
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.01);
        env.set_decay(0.1);
        env.set_sustain(0.7);
        env.set_release(0.2);

        let start_sample = (start * sr) as usize;
        let end_sample = ((start + dur) * sr) as usize;
        let release_sample = end_sample;
        let total_end = ((start + dur + 0.3) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let out = osc.process() * env_val * 0.3;
            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_analog_square(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = AnalogOscillator::new(sr);
        osc.set_waveform(Waveform::Square);
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.02);
        env.set_decay(0.15);
        env.set_sustain(0.6);
        env.set_release(0.25);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.4) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let out = osc.process() * env_val * 0.25;
            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_supersaw(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = MultiOscillator::new(sr);
        osc.set_wavetable(1); // FatSaw
        osc.set_frequency(midi_to_freq(note));
        osc.set_detune(0.6);
        osc.set_stereo_width(0.8);
        osc.randomize_phase();

        let mut env = Adsr::new(sr);
        env.set_attack(0.05);
        env.set_decay(0.2);
        env.set_sustain(0.8);
        env.set_release(0.4);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.5) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let (left, right) = osc.process_stereo();
            samples[i * 2] += left * env_val * 0.25;
            samples[i * 2 + 1] += right * env_val * 0.25;
        }
    }
}

fn generate_wavetable_pad(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = WavetableOscillator::new(sr);
        osc.set_wavetable(5);
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.3);
        env.set_decay(0.2);
        env.set_sustain(0.7);
        env.set_release(0.8);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 1.0) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let out = osc.process() * env_val * 0.3;
            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_wavetable2d(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = WavetableOsc2D::new(sr);
        osc.set_preset(0); // Basic (Saw->Square->Tri->Sine)
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.1);
        env.set_decay(0.2);
        env.set_sustain(0.6);
        env.set_release(0.5);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.6) * sr) as usize;
        let note_duration = total_end - start_sample;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            // Sweep position during note
            let t = (i - start_sample) as f32 / note_duration as f32;
            osc.set_position(t);

            let env_val = env.process();
            let out = osc.process() * env_val * 0.35;
            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_filtered_bass(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, dur) in melody.notes {
        let mut osc = AnalogOscillator::new(sr);
        osc.set_waveform(Waveform::Saw);
        osc.set_frequency(midi_to_freq(note));

        let mut filter = LadderFilter::new(sr);
        filter.set_filter_type(LadderFilterType::LP4);
        filter.set_resonance(0.4);

        let mut amp_env = Adsr::new(sr);
        amp_env.set_attack(0.005);
        amp_env.set_decay(0.2);
        amp_env.set_sustain(0.5);
        amp_env.set_release(0.15);

        let mut filter_env = Adsr::new(sr);
        filter_env.set_attack(0.005);
        filter_env.set_decay(0.3);
        filter_env.set_sustain(0.2);
        filter_env.set_release(0.1);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.3) * sr) as usize;

        amp_env.trigger();
        filter_env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                amp_env.release();
                filter_env.release();
            }
            let amp_val = amp_env.process();
            let filter_val = filter_env.process();

            // Filter envelope modulates cutoff
            let cutoff = 100.0 + filter_val * 2000.0;
            filter.set_cutoff(cutoff);

            let osc_out = osc.process();
            let filtered = filter.process(osc_out);
            let out = filtered * amp_val * 0.4;

            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_pluck(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    for &(note, start, _dur) in melody.notes {
        let mut osc = WavetableOscillator::new(sr);
        osc.set_wavetable(2); // Triangle-like
        osc.set_frequency(midi_to_freq(note));

        let mut filter = LadderFilter::new(sr);
        filter.set_filter_type(LadderFilterType::LP4);
        filter.set_resonance(0.3);

        let mut env = Adsr::new(sr);
        env.set_attack(0.001);
        env.set_decay(0.15);
        env.set_sustain(0.0);
        env.set_release(0.1);

        let start_sample = (start * sr) as usize;
        let total_end = ((start + 0.3) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            let env_val = env.process();

            // Fast filter decay
            let cutoff = 500.0 + env_val * 4000.0;
            filter.set_cutoff(cutoff);

            let osc_out = osc.process();
            let filtered = filter.process(osc_out);
            let out = filtered * env_val * 0.4;

            samples[i * 2] += out;
            samples[i * 2 + 1] += out;
        }
    }
}

fn generate_pad_reverb(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    let mut reverb = ZitaReverb::new(sr);
    reverb.set_mix(0.5);
    reverb.set_rtmid(3.0);
    reverb.set_fdamp(5000.0);

    // First generate dry signal
    let mut dry = vec![0.0f32; num_samples];

    for &(note, start, dur) in melody.notes {
        let mut osc = MultiOscillator::new(sr);
        osc.set_wavetable(5);
        osc.set_frequency(midi_to_freq(note));
        osc.set_detune(0.3);

        let mut env = Adsr::new(sr);
        env.set_attack(0.4);
        env.set_decay(0.3);
        env.set_sustain(0.6);
        env.set_release(1.0);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 1.2) * sr) as usize;

        env.trigger();

        for (i, d) in dry.iter_mut().enumerate().take(total_end.min(num_samples)).skip(start_sample) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            *d += osc.process() * env_val * 0.2;
        }
    }

    // Apply reverb
    for i in 0..num_samples {
        let (left, right) = reverb.process(dry[i], dry[i]);
        samples[i * 2] = left;
        samples[i * 2 + 1] = right;
    }
}

fn generate_delay_lead(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    let mut delay = Delay::new(sr);
    delay.set_delay_time(0.375); // Dotted eighth
    delay.set_feedback(0.4);
    delay.set_wet(0.35);
    delay.set_dry(0.65);

    for &(note, start, dur) in melody.notes {
        let mut osc = AnalogOscillator::new(sr);
        osc.set_waveform(Waveform::Saw);
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.02);
        env.set_decay(0.1);
        env.set_sustain(0.6);
        env.set_release(0.2);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.3) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let dry = osc.process() * env_val * 0.3;
            let (left, right) = delay.process(dry, dry);
            samples[i * 2] += left;
            samples[i * 2 + 1] += right;
        }
    }
}

fn generate_chorus_strings(melody: &Melody, samples: &mut [f32]) {
    let sr = SAMPLE_RATE as f32;
    let num_samples = samples.len() / 2;

    let mut chorus = Chorus::new(sr);
    chorus.set_amount(0.6);
    chorus.set_dry_wet(0.5);
    chorus.set_lfo_freq(0.3);

    for &(note, start, dur) in melody.notes {
        let mut osc = WavetableOscillator::new(sr);
        osc.set_wavetable(1); // FatSaw
        osc.set_frequency(midi_to_freq(note));

        let mut env = Adsr::new(sr);
        env.set_attack(0.2);
        env.set_decay(0.1);
        env.set_sustain(0.8);
        env.set_release(0.5);

        let start_sample = (start * sr) as usize;
        let release_sample = ((start + dur) * sr) as usize;
        let total_end = ((start + dur + 0.6) * sr) as usize;

        env.trigger();

        for i in start_sample..total_end.min(num_samples) {
            if i == release_sample {
                env.release();
            }
            let env_val = env.process();
            let dry = osc.process() * env_val * 0.25;
            let (left, right) = chorus.process(dry);
            samples[i * 2] += left;
            samples[i * 2 + 1] += right;
        }
    }
}

fn print_usage() {
    println!("Odin 2 Demo - WAV Generator");
    println!();
    println!("Usage: odin2-demo [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --list              List all available presets and melodies");
    println!("  --preset NAME       Use a specific preset (default: all)");
    println!("  --melody NAME       Use a specific melody (default: arpeggio)");
    println!("  --output DIR        Output directory (default: ../../samples/demos)");
    println!("  --play              Play audio after generation (macOS only)");
    println!("  --factory NAME      Load and render a factory preset by name");
    println!("  --factory-file PATH Load and render a .odin preset file");
    println!("  --list-factory      List all factory presets");
    println!("  --help              Show this help");
    println!();
    println!("Examples:");
    println!("  odin2-demo --list");
    println!("  odin2-demo --preset supersaw --melody lead_melody");
    println!("  odin2-demo --preset supersaw --play");
    println!("  odin2-demo --list-factory");
    println!("  odin2-demo --factory \"Analog Bass\" --melody bass_line --play");
    println!("  odin2-demo --output ./my_sounds");
}

fn print_list() {
    println!("Available Presets:");
    println!("------------------");
    for p in PRESETS {
        println!("  {:20} - {}", p.name, p.description);
    }
    println!();
    println!("Available Melodies:");
    println!("-------------------");
    for m in MELODIES {
        println!(
            "  {:20} - {} notes, {:.1}s",
            m.name,
            m.notes.len(),
            m.duration
        );
    }
}

/// Factory presets base path
const FACTORY_PRESETS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../odin2/assets/Soundbanks/Factory Presets");

/// List all factory presets
fn print_factory_list() {
    let base_path = Path::new(FACTORY_PRESETS_PATH);
    if !base_path.exists() {
        eprintln!("Factory presets not found at: {}", FACTORY_PRESETS_PATH);
        return;
    }

    println!("Factory Presets:");
    println!("================");

    if let Ok(entries) = fs::read_dir(base_path) {
        let mut categories: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();
        categories.sort_by_key(|e| e.file_name());

        for entry in categories {
            let category = entry.file_name().to_string_lossy().to_string();
            println!("\n{}:", category);
            println!("{}", "-".repeat(category.len() + 1));

            if let Ok(files) = fs::read_dir(entry.path()) {
                let mut presets: Vec<_> = files
                    .flatten()
                    .filter(|f| f.path().extension().is_some_and(|ext| ext == "odin"))
                    .collect();
                presets.sort_by_key(|f| f.file_name());

                for file in presets {
                    let name = file.path().file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    println!("  {}", name);
                }
            }
        }
    }
}

/// Find a factory preset by name (searches all categories)
fn find_factory_preset(name: &str) -> Option<std::path::PathBuf> {
    let base_path = Path::new(FACTORY_PRESETS_PATH);
    if !base_path.exists() {
        return None;
    }

    let search_name = name.to_lowercase();

    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Ok(files) = fs::read_dir(entry.path()) {
                    for file in files.flatten() {
                        if file.path().extension().is_some_and(|ext| ext == "odin") {
                            let preset_name = file.path().file_stem()
                                .map(|s| s.to_string_lossy().to_string().to_lowercase())
                                .unwrap_or_default();

                            if preset_name.contains(&search_name) {
                                return Some(file.path());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Rendu d'un patch complet par le même moteur que l'hôte.
fn generate_from_odin_preset(preset: &OdinPreset, melody: &Melody) -> Vec<f32> {
    let sr = SAMPLE_RATE as f32;
    let frames = (sr * melody.duration) as usize;
    let mut samples = vec![0.0; frames * 2];
    let mut engine = OdinEngine::new(sr);
    engine.load_preset(preset).expect("Matrice de modulation du preset invalide");
    let mut events = Vec::with_capacity(melody.notes.len() * 2);
    for &(note, start, duration) in melody.notes {
        events.push(((start * sr) as usize, true, note));
        events.push((((start + duration) * sr) as usize, false, note));
    }
    events.sort_by_key(|&(at, on, _)| (at, on));
    let mut cursor = 0;
    for (at, on, note) in events {
        let at = at.min(frames);
        engine.process(&mut samples[cursor * 2..at * 2], 2);
        if on { engine.note_on(note, 100); } else { engine.note_off(note); }
        cursor = at;
    }
    engine.process(&mut samples[cursor * 2..], 2);
    samples
}


fn main() {
    let args: Vec<String> = env::args().collect();

    let mut preset_filter: Option<String> = None;
    let mut factory_preset: Option<String> = None;
    let mut factory_file: Option<String> = None;
    let mut melody_name = "arpeggio".to_string();
    let mut output_dir = "../../samples/demos".to_string();
    let mut play_audio = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "--list" | "-l" => {
                print_list();
                return;
            }
            "--list-factory" => {
                print_factory_list();
                return;
            }
            "--preset" | "-p" => {
                i += 1;
                if i < args.len() {
                    preset_filter = Some(args[i].clone());
                }
            }
            "--factory" | "-f" => {
                i += 1;
                if i < args.len() {
                    factory_preset = Some(args[i].clone());
                }
            }
            "--factory-file" => {
                i += 1;
                if i < args.len() {
                    factory_file = Some(args[i].clone());
                }
            }
            "--melody" | "-m" => {
                i += 1;
                if i < args.len() {
                    melody_name = args[i].clone();
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_dir = args[i].clone();
                }
            }
            "--play" => {
                play_audio = true;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                return;
            }
        }
        i += 1;
    }

    // Find melody
    let melody = MELODIES
        .iter()
        .find(|m| m.name == melody_name)
        .unwrap_or_else(|| {
            eprintln!("Unknown melody: {}. Using 'arpeggio'.", melody_name);
            &MELODIES[0]
        });

    // Create output directory
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let mut generated_files = Vec::new();

    // Handle factory preset mode
    if factory_preset.is_some() || factory_file.is_some() {
        let odin_preset = if let Some(ref name) = factory_preset {
            // Find preset by name
            match find_factory_preset(name) {
                Some(path) => {
                    println!("Found factory preset: {}", path.display());
                    match OdinPreset::load(&path) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("Error loading preset: {}", e);
                            return;
                        }
                    }
                }
                None => {
                    eprintln!("Factory preset '{}' not found.", name);
                    eprintln!("Use --list-factory to see available presets.");
                    return;
                }
            }
        } else if let Some(ref path) = factory_file {
            // Load from file path
            match OdinPreset::load(path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error loading preset file '{}': {}", path, e);
                    return;
                }
            }
        } else {
            unreachable!()
        };

        println!("Generating from Odin preset: '{}'", odin_preset.name);
        println!("  Osc1: {:?} ({:?})", odin_preset.osc1.osc_type, odin_preset.osc1.analog_wave);
        println!("  Filter1: {:?} @ {:.0} Hz", odin_preset.filter1.filter_type, odin_preset.filter1.frequency);
        println!("  Effects: delay={}, reverb={}, chorus={}",
                 odin_preset.delay.on, odin_preset.reverb.on, odin_preset.chorus.on);
        println!();

        print!("  Rendering with melody '{}' ... ", melody.name);

        let samples = generate_from_odin_preset(&odin_preset, melody);
        let safe_name = odin_preset.name.replace(' ', "_").replace(['[', ']', '(', ')'], "");
        let filename = format!("{}/{}_{}.wav", output_dir, safe_name, melody.name);
        write_stereo_wav(&filename, &samples);

        println!("OK -> {}", filename);
        generated_files.push(filename);
    } else {
        // Standard preset mode
        let presets_to_run: Vec<&Preset> = if let Some(ref name) = preset_filter {
            PRESETS
                .iter()
                .filter(|p| p.name == name)
                .collect()
        } else {
            PRESETS.iter().collect()
        };

        if presets_to_run.is_empty() {
            eprintln!("No matching presets found.");
            return;
        }

        println!("Generating {} preset(s) with melody '{}'...", presets_to_run.len(), melody.name);
        println!();

        for preset in presets_to_run {
            print!("  {} ... ", preset.name);

            let samples = generate_preset(preset.name, melody);
            let filename = format!("{}/{}_{}.wav", output_dir, preset.name, melody.name);
            write_stereo_wav(&filename, &samples);

            println!("OK -> {}", filename);
            generated_files.push(filename);
        }
    }

    println!();
    println!("Done! Generated files in '{}'", output_dir);

    // Play audio if requested
    if play_audio && !generated_files.is_empty() {
        println!();
        println!("Playing generated audio...");
        for file in &generated_files {
            println!("  Playing: {}", file);
            let status = Command::new("afplay")
                .arg(file)
                .status();

            match status {
                Ok(s) if s.success() => {}
                Ok(_) => eprintln!("  Warning: afplay exited with non-zero status"),
                Err(e) => {
                    eprintln!("  Error: Could not play audio: {}", e);
                    eprintln!("  Note: --play requires macOS with afplay command");
                    break;
                }
            }
        }
    }
}
