//! Odin 2 Preset Morphing Demo
//!
//! Demonstrates smooth morphing between factory presets.
//! This is a proof-of-concept for real-time preset interpolation in Harmonium.
//!
//! Usage:
//!   cargo run --bin morph-demo --features std -- [OPTIONS]

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use hound::{WavSpec, WavWriter};

use odin2_core::{EngineConfig, OdinEngine, ParameterId, SynthEngine};
use odin2_core::preset::OdinPreset;

const SAMPLE_RATE: u32 = 44100;

/// Melody definition
struct Melody {
    name: &'static str,
    notes: &'static [(u8, f32, f32)], // (midi_note, start_time, duration)
}

/// Available melodies for morphing
const MELODIES: &[Melody] = &[
    Melody {
        name: "slow_arpeggio",
        notes: &[
            (48, 0.0, 0.8),   // C3
            (52, 1.0, 0.8),   // E3
            (55, 2.0, 0.8),   // G3
            (60, 3.0, 0.8),   // C4
            (64, 4.0, 0.8),   // E4
            (67, 5.0, 0.8),   // G4
            (72, 6.0, 0.8),   // C5
            (67, 7.0, 0.8),   // G4
            (64, 8.0, 0.8),   // E4
            (60, 9.0, 0.8),   // C4
            (55, 10.0, 0.8),  // G3
            (48, 11.0, 1.5),  // C3 (long)
        ],
    },
    Melody {
        name: "chord_stabs",
        notes: &[
            // Chord 1 - C major
            (48, 0.0, 0.6), (60, 0.0, 0.6), (64, 0.0, 0.6), (67, 0.0, 0.6),
            // Chord 2 - F major
            (53, 2.0, 0.6), (60, 2.0, 0.6), (65, 2.0, 0.6), (69, 2.0, 0.6),
            // Chord 3 - Am
            (45, 4.0, 0.6), (57, 4.0, 0.6), (60, 4.0, 0.6), (64, 4.0, 0.6),
            // Chord 4 - G
            (43, 6.0, 0.6), (55, 6.0, 0.6), (59, 6.0, 0.6), (67, 6.0, 0.6),
            // Chord 5 - C major (long)
            (48, 8.0, 2.0), (60, 8.0, 2.0), (64, 8.0, 2.0), (67, 8.0, 2.0),
        ],
    },
    Melody {
        name: "lead_line",
        notes: &[
            (60, 0.0, 0.3),   // C4
            (62, 0.5, 0.3),   // D4
            (64, 1.0, 0.5),   // E4
            (67, 1.75, 0.25), // G4
            (72, 2.0, 0.8),   // C5
            (71, 3.0, 0.3),   // B4
            (69, 3.5, 0.3),   // A4
            (67, 4.0, 0.8),   // G4
            (64, 5.0, 0.3),   // E4
            (62, 5.5, 0.3),   // D4
            (60, 6.0, 0.5),   // C4
            (64, 6.75, 0.25), // E4
            (67, 7.0, 0.5),   // G4
            (72, 7.75, 0.25), // C5
            (76, 8.0, 1.5),   // E5 (long)
        ],
    },
    Melody {
        name: "bass_groove",
        notes: &[
            (36, 0.0, 0.3),   // C2
            (36, 0.5, 0.2),   // C2
            (36, 1.0, 0.3),   // C2
            (38, 1.5, 0.2),   // D2
            (40, 2.0, 0.3),   // E2
            (40, 2.5, 0.2),   // E2
            (43, 3.0, 0.3),   // G2
            (40, 3.5, 0.2),   // E2
            (36, 4.0, 0.3),   // C2
            (36, 4.5, 0.2),   // C2
            (36, 5.0, 0.3),   // C2
            (38, 5.5, 0.2),   // D2
            (40, 6.0, 0.3),   // E2
            (43, 6.5, 0.2),   // G2
            (48, 7.0, 0.8),   // C3
            (36, 8.0, 1.5),   // C2 (long)
        ],
    },
    Melody {
        name: "sustained",
        notes: &[
            // Just one long sustained chord
            (48, 0.0, 10.0), // C3
            (52, 0.0, 10.0), // E3
            (55, 0.0, 10.0), // G3
            (59, 0.0, 10.0), // B3
        ],
    },
];

/// Factory presets base path
const FACTORY_PRESETS_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../odin2/assets/Soundbanks/Factory Presets");


/// Morph sequence definition
struct MorphSequence {
    presets: Vec<OdinPreset>,
    params: Vec<EngineConfig>,
    durations: Vec<f32>, // Duration for each morph transition in seconds
}

impl MorphSequence {
    fn new() -> Self {
        Self {
            presets: Vec::new(),
            params: Vec::new(),
            durations: Vec::new(),
        }
    }

    fn add_preset(&mut self, preset: OdinPreset, transition_duration: f32) {
        self.params.push(EngineConfig::from_preset(&preset));
        self.presets.push(preset);
        if !self.durations.is_empty() || self.params.len() > 1 {
            self.durations.push(transition_duration);
        }
    }

    fn total_duration(&self) -> f32 {
        self.durations.iter().sum()
    }

    /// Interpolation du catalogue entier ; topologie du preset le plus proche.
    fn get_params_at(&self, time: f32) -> (EngineConfig, usize) {
        for (index, &duration) in self.durations.iter().enumerate() {
            let begin: f32 = self.durations[..index].iter().sum();
            if time < begin + duration {
                let t = ((time - begin) / duration).clamp(0.0, 1.0);
                let t = t * t * (3.0 - 2.0 * t);
                let topology = if t < 0.5 { index } else { index + 1 };
                let mut config = self.params[topology];
                for parameter in ParameterId::all() {
                    let a = parameter.get(&self.params[index]);
                    let b = parameter.get(&self.params[index + 1]);
                    parameter.set(&mut config, a + (b - a) * t).expect("Paramètre interpolé invalide");
                }
                return (config, topology);
            }
        }
        (*self.params.last().expect("Séquence de presets vide"), self.params.len() - 1)
    }
}


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
                            let preset_name = file
                                .path()
                                .file_stem()
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

fn load_preset(name: &str) -> Option<OdinPreset> {
    find_factory_preset(name).and_then(|path| {
        println!("  Loading: {} -> {}", name, path.display());
        OdinPreset::load(&path).ok()
    })
}


/// Generate morphing audio with a melody
fn generate_morph_audio(sequence: &MorphSequence, melody: &Melody) -> Vec<f32> {
    let sr = SAMPLE_RATE as f32;
    let duration = sequence.total_duration();
    let frames = (sr * duration) as usize;
    let mut samples = vec![0.0; frames * 2];
    let mut engine = OdinEngine::new(sr);
    engine.load_config(sequence.params[0]).expect("Preset initial invalide");
    let melody_duration = melody.notes.iter().map(|&(_, start, length)| start + length).fold(0.0_f32, f32::max);
    let scale = duration / melody_duration;
    let mut events = Vec::with_capacity(melody.notes.len() * 2);
    for &(note, start, length) in melody.notes {
        events.push(((start * scale * sr) as usize, true, note));
        events.push((((start + length) * scale * sr) as usize, false, note));
    }
    events.sort_by_key(|&(at, on, _)| (at, on));
    let mut values: Vec<_> = ParameterId::all().map(|id| (id, 0.0)).collect();
    let mut event = 0;
    let mut held = [false; 128];
    let mut topology = 0;
    for i in 0..frames {
        // Cette démo est un rendu hors ligne. Aucune seconde implémentation DSP.
        if i % 64 == 0 {
            let (config, next_topology) = sequence.get_params_at(i as f32 / sr);
            if next_topology != topology {
                engine.load_config(config).expect("Preset interpolé invalide");
                for (note, &active) in held.iter().enumerate() {
                    if active { engine.note_on(note as u8, 100); }
                }
                topology = next_topology;
            } else {
                for (id, value) in &mut values { *value = id.get(&config); }
                engine.set_parameters(&values).expect("Paramètres interpolés invalides");
            }
        }
        while event < events.len() && events[event].0 <= i {
            let (_, on, note) = events[event];
            held[note as usize] = on;
            if on { engine.note_on(note, 100); } else { engine.note_off(note); }
            event += 1;
        }
        engine.process(&mut samples[i * 2..i * 2 + 2], 2);
    }
    samples
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

fn print_usage() {
    println!("Odin 2 Preset Morphing Demo");
    println!();
    println!("Usage: morph-demo [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --demo N            Run built-in demo N (1-3)");
    println!("  --presets A,B,C     Morph between named presets (comma-separated)");
    println!("  --melody NAME       Melody to play (default: slow_arpeggio)");
    println!("  --duration SECS     Duration per morph transition (default: 4.0)");
    println!("  --output FILE       Output WAV file (default: ../../samples/demos/morph_output.wav)");
    println!("  --play              Play audio after generation");
    println!("  --help              Show this help");
    println!();
    println!("Built-in Demos:");
    println!("  1: Pad Evolution    - Warm Pad → Synthwave Pad → Eerie Pad");
    println!("  2: Soft to Harsh    - Synth Strings → Lead Sine → Lead Square");
    println!("  3: Atmospheric      - Frosty Atmo → Drifter → Moonshade");
    println!();
    println!("Available Melodies:");
    println!("  slow_arpeggio  - Ascending/descending arpeggio (12 notes)");
    println!("  chord_stabs    - Chord progression stabs (C-F-Am-G-C)");
    println!("  lead_line      - Melodic lead line (15 notes)");
    println!("  bass_groove    - Bass line groove (16 notes)");
    println!("  sustained      - One long sustained chord");
    println!();
    println!("Examples:");
    println!("  morph-demo --demo 1 --play");
    println!("  morph-demo --demo 1 --melody lead_line --play");
    println!("  morph-demo --presets \"Warm-Pad,Lead Sine\" --melody chord_stabs --play");
    println!("  morph-demo --presets \"Analog Bass,Lead Rusty\" --melody bass_groove --duration 5 --play");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut demo_num: Option<u32> = None;
    let mut preset_names: Option<String> = None;
    let mut melody_name = "slow_arpeggio".to_string();
    let mut duration = 4.0f32;
    let mut output_file = "../../samples/demos/morph_output.wav".to_string();
    let mut play_audio = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "--demo" | "-d" => {
                i += 1;
                if i < args.len() {
                    demo_num = args[i].parse().ok();
                }
            }
            "--presets" | "-p" => {
                i += 1;
                if i < args.len() {
                    preset_names = Some(args[i].clone());
                }
            }
            "--melody" | "-m" => {
                i += 1;
                if i < args.len() {
                    melody_name = args[i].clone();
                }
            }
            "--duration" | "-t" => {
                i += 1;
                if i < args.len() {
                    duration = args[i].parse().unwrap_or(4.0);
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = args[i].clone();
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

    // Determine which presets to load
    let preset_list: Vec<&str> = if let Some(ref names) = preset_names {
        names.split(',').map(|s| s.trim()).collect()
    } else if let Some(num) = demo_num {
        match num {
            1 => vec!["Warm-Pad", "Synthwave Pad", "Eerie Pad"],
            2 => vec!["Synth Strings", "Lead Sine", "Lead Square"],
            3 => vec!["Frosty Atmo", "Drifter", "Moonshade"],
            _ => {
                eprintln!("Unknown demo number: {}. Use 1, 2, or 3.", num);
                return;
            }
        }
    } else {
        // Default demo
        println!("No demo or presets specified. Running Demo 1 (Pad Evolution).");
        println!();
        vec!["Warm-Pad", "Synthwave Pad", "Eerie Pad"]
    };

    if preset_list.len() < 2 {
        eprintln!("Need at least 2 presets for morphing.");
        return;
    }

    println!("=== Odin 2 Preset Morphing Demo ===");
    println!();
    println!("Loading presets:");

    let mut sequence = MorphSequence::new();
    for (i, name) in preset_list.iter().enumerate() {
        match load_preset(name) {
            Some(preset) => {
                println!("    ✓ '{}' ({:?}, filter @ {:.0} Hz)",
                         preset.name, preset.osc1.osc_type, preset.filter1.frequency);
                let trans_duration = if i == 0 { 0.0 } else { duration };
                sequence.add_preset(preset, trans_duration);
            }
            None => {
                eprintln!("    ✗ Preset '{}' not found!", name);
                return;
            }
        }
    }

    // Find melody
    let melody = MELODIES
        .iter()
        .find(|m| m.name == melody_name)
        .unwrap_or_else(|| {
            eprintln!("Unknown melody: '{}'. Using 'slow_arpeggio'.", melody_name);
            &MELODIES[0]
        });

    println!();
    println!("Morph sequence:");
    for (i, preset) in sequence.presets.iter().enumerate() {
        if i > 0 {
            println!("    ↓ ({:.1}s morph)", sequence.durations[i - 1]);
        }
        println!("  [{}] {}", i + 1, preset.name);
    }
    println!();
    println!("Total duration: {:.1}s", sequence.total_duration());
    println!("Melody: {} ({} notes)", melody.name, melody.notes.len());
    println!();

    let samples = generate_morph_audio(&sequence, melody);

    println!("  Writing to: {}", output_file);
    write_stereo_wav(&output_file, &samples);

    println!();
    println!("Done!");

    if play_audio {
        println!();
        println!("Playing audio...");
        let status = Command::new("afplay").arg(&output_file).status();

        match status {
            Ok(s) if s.success() => {}
            Ok(_) => eprintln!("Warning: afplay exited with non-zero status"),
            Err(e) => {
                eprintln!("Error: Could not play audio: {}", e);
                eprintln!("Note: --play requires macOS with afplay command");
            }
        }
    }
}
