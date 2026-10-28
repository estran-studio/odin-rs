//! Preset Morphing Example
//!
//! Demonstrates smooth morphing between Odin 2 presets coordinates on a 2D plane.
//!
//! Usage:
//!   cargo run --example preset_morph --features std
//!   cargo run --example preset_morph --features std -- --generate-audio

use odin2_core::preset::OdinPreset;
use odin2_core::engine::{OdinEngine, SynthEngine};
use hound::{WavSpec, WavWriter};
use std::env;

fn main() {
    println!("=== Odin 2 Scatter Morphing Example ===\n");

    // Check if we should generate audio files
    let args: Vec<String> = env::args().collect();
    let generate_audio = args.iter().any(|arg| arg == "--generate-audio");

    // Create engine
    let sample_rate = 44100.0;
    let mut engine = OdinEngine::new(sample_rate);

    println!("Loading factory source presets...\n");

    let path_prefix = "odin2/assets/Soundbanks/Factory Presets/Keys";

    // Preset 1: Synth Piano
    let mut p1 = OdinPreset::load(format!("{}/Synth Piano.odin", path_prefix))
        .expect("Failed to load Synth Piano");
    p1.name = "P1: Synth Piano".to_string();
    println!("✓ Loaded: {}", p1.name);
    println!("  Master: {}, Amp Gain: {}", p1.master, p1.amp_gain);
    println!("  Osc1: Vol={}, Type={:?}", p1.osc1.volume, p1.osc1.osc_type);
    println!("  Osc2: Vol={}, Type={:?}", p1.osc2.volume, p1.osc2.osc_type);
    println!("  Osc3: Vol={}, Type={:?}", p1.osc3.volume, p1.osc3.osc_type);
    println!("  Filter1 Freq: {}", p1.filter1.frequency);
    println!("  Amp Env: A={}, D={}, S={}, R={}", p1.env1.attack, p1.env1.decay, p1.env1.sustain, p1.env1.release);

    // Preset 2: Toy Piano
    let mut p2 = OdinPreset::load(format!("{}/Toy Piano.odin", path_prefix))
        .expect("Failed to load Toy Piano");
    p2.name = "P2: Toy Piano".to_string();
    println!("✓ Loaded: {}", p2.name);

    // Preset 3: Pianet
    let mut p3 = OdinPreset::load(format!("{}/Pianet.odin", path_prefix))
        .expect("Failed to load Pianet");
    p3.name = "P3: Pianet".to_string();
    println!("✓ Loaded: {}", p3.name);

    // Preset 4: Piano Ballad 3
    let mut p4 = OdinPreset::load(format!("{}/Piano Ballad 3.odin", path_prefix))
        .expect("Failed to load Piano Ballad 3");
    p4.name = "P4: Piano Ballad".to_string();
    println!("✓ Loaded: {}", p4.name);

    // Define sources with their coordinates
    let sources = vec![
        (p1.clone(), -1.0, 1.0),
        (p2.clone(), 1.0, 1.0),
        (p3.clone(), -1.0, -1.0),
        (p4.clone(), 1.0, -1.0),
    ];

    println!("\n--- Morphing Examples ---\n");

    // Example 1: Center (0, 0)
    println!("1. Center Position (0.0, 0.0):");
    let morphed_center = OdinPreset::morph_2d(&sources, 0.0, 0.0);
    println!("   Name: {}", morphed_center.name); // Should be blend of all 4
    println!("   Filter freq: {:.1} Hz", morphed_center.filter1.frequency);
    println!("   Env1 attack: {:.4}s", morphed_center.env1.attack);

    // Example 2: Near P1 (-0.8, 0.8)
    println!("\n2. Near P1 (-0.8, 0.8):");
    let morphed_p1 = OdinPreset::morph_2d(&sources, -0.8, 0.8);
    println!("   Filter freq: {:.1} Hz (Close to 8000)", morphed_p1.filter1.frequency);

    // Example 3: Near P3 (-0.8, -0.8)
    println!("\n3. Near P3 (-0.8, -0.8):");
    let morphed_p3 = OdinPreset::morph_2d(&sources, -0.8, -0.8);
    println!("   Filter freq: {:.1} Hz (Close to 120)", morphed_p3.filter1.frequency);

    // Load morphed preset into engine
    println!("\n--- Loading Morphed Preset into Engine ---");
    engine.load_preset(&morphed_center).expect("Matrice de modulation du preset invalide");
    println!("✓ Loaded center blend into engine");

    // Generate audio if requested
    if generate_audio {
        println!("\n--- Generating Source Preset Previews ---");
        // Verify individual presets work before morphing
        generate_preview("preview_p1_synth_piano.wav", sample_rate, &p1);
        generate_preview("preview_p2_toy_piano.wav", sample_rate, &p2);
        generate_preview("preview_p3_pianet.wav", sample_rate, &p3);
        generate_preview("preview_p4_piano_ballad.wav", sample_rate, &p4);

        println!("\n--- Generating Morphing Audio Files ---");
        // Linear Transitions
        // 1. Sad to Happy: Synth Piano -> Toy Piano
        generate_linear_transition("transition_synth_to_toy.wav", sample_rate, &p1, &p2);
        
        // 2. Pianet -> Piano Ballad
        generate_linear_transition("transition_pianet_to_ballad.wav", sample_rate, &p3, &p4);

        // 3. Diagonal Morph (True 2D Test)
        // Moves from Top-Left (Synth Piano) to Bottom-Right (Piano Ballad)
        // traversing the center where all 4 presets blend.
        generate_path_morph(
            "morph_diagonal_2d.wav", 
            sample_rate, 
            &sources, 
            (-1.0, 1.0), // Start: Top-Left
            (1.0, -1.0)  // End: Bottom-Right
        );

        // 4. Circular Morph
        // Rotates around the center at radius 0.8, visiting all quadrants
        generate_circle_morph("morph_circle.wav", sample_rate, &sources, 0.8);

        println!("\n--- Generating Drum Morphing Demos ---");
        run_drum_morph_demo(sample_rate);

    } else {
        // Quick test
        println!("\n--- Quick Audio Test ---");
        println!("Playing note C4 (MIDI 60) for 1000 samples...");

        engine.note_on(60, 100);
        let mut buffer = vec![0.0f32; 2000]; // 1000 stereo samples
        engine.process(&mut buffer, 2);

        // Check if we got audio output
        let max_amplitude = buffer.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
        println!("Max amplitude: {:.4}", max_amplitude);

        if max_amplitude > 0.0001 {
            println!("✓ Audio generated successfully!");
        } else {
            println!("⚠ Warning: Audio output is very quiet or silent");
        }

        println!("\n💡 Tip: Run with --generate-audio to create WAV files you can listen to:");
        println!("   cargo run --example preset_morph --features std -- --generate-audio");
    }

    println!("\n✓ Done!");
}

/// Generate a simple preview of a single preset
fn generate_preview(filename: &str, sample_rate: f32, preset: &OdinPreset) {
    println!("Generating preview: {} ({})", preset.name, filename);
    let output_dir = "samples/morphing";
    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");
    
    let mut engine = OdinEngine::new(sample_rate);
    engine.load_preset(preset).expect("Matrice de modulation du preset invalide");

    // Play a C Major Arpeggio
    let notes = [60, 64, 67, 72];
    let note_duration = 0.5; // seconds
    let samples_per_note = (sample_rate * note_duration) as usize;
    
    let mut buffer = Vec::new();

    for &note in &notes {
        engine.note_on(note, 100);
        
        let mut note_buffer = vec![0.0f32; samples_per_note * 2]; // Stereo
        engine.process(&mut note_buffer, 2);
        
        buffer.extend_from_slice(&note_buffer);
        engine.note_off(note);
    }

    // Release phase
    let release_samples = (sample_rate * 1.0) as usize;
    let mut release_buffer = vec![0.0f32; release_samples * 2];
    engine.process(&mut release_buffer, 2);
    buffer.extend_from_slice(&release_buffer);

    // Check Max Amplitude
    let max_amp = buffer.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
    println!("  -> Max Amplitude: {:.6}", max_amp);
    
    if max_amp < 0.001 {
        println!("  WARNING: Signal is very weak!");
    }

    let wav_path = format!("{}/{}", output_dir, filename);
    write_wav(&wav_path, &buffer, sample_rate as u32);
    println!("  ✓ Saved to {}", wav_path);
}

/// Write stereo audio to WAV file
fn write_wav(path: &str, samples: &[f32], sample_rate: u32) {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
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

/// Generate a specific demo transitioning from "Sad" to "Happy" over time
fn generate_linear_transition(
    filename: &str,
    sample_rate: f32,
    start_preset: &OdinPreset,
    end_preset: &OdinPreset
) {
    println!("\nGenerating transition: {} -> {} ({})", start_preset.name, end_preset.name, filename);
    let output_dir = "samples/morphing";
    
    // 2. Setup Engine
    let mut engine = OdinEngine::new(sample_rate);
    
    // 3. Define Sequence (Fast arpeggio to show per-note timbral change)
    let duration = 10.0; // seconds
    let note_duration = 0.125; // 8th notes at 120bpm approx
    let num_notes = (duration / note_duration) as usize;
    
    // Simple progression
    let notes = [60, 64, 67, 72, 67, 64]; // C Major Arpeggio
    
    let mut buffer = Vec::new();
    let samples_per_note = (sample_rate * note_duration) as usize;
    
    for i in 0..num_notes {
        let t = i as f32 / num_notes as f32; // 0.0 to 1.0
        
        // Interpolate preset
        let current_preset = start_preset.interpolate(end_preset, t);
        
        // Chargement complet hors callback : arrête les voix du patch précédent.
        engine.load_preset(&current_preset).expect("Matrice de modulation du preset invalide");
        
        // Play note
        let note = notes[i % notes.len()];
        engine.note_on(note, 100);
        
        // Render block
        let mut note_buffer = vec![0.0f32; samples_per_note * 2];
        for j in 0..samples_per_note {
            let mut frame = [0.0; 2];
            engine.process(&mut frame, 2);
            note_buffer[j*2] = frame[0];
            note_buffer[j*2+1] = frame[1];
        }
        
        // Note off (shortly before end for articulation)
        engine.note_off(note);
        
        buffer.extend_from_slice(&note_buffer);
    }
    
    // Write File
    let wav_path = format!("{}/{}", output_dir, filename);
    write_wav(&wav_path, &buffer, sample_rate as u32);
    println!("  ✓ Saved to {}", wav_path);
}

/// Generates a morph along a 2D path using the scatter system
fn generate_path_morph(
    filename: &str, 
    sample_rate: f32, 
    sources: &[(OdinPreset, f32, f32)],
    start_pos: (f32, f32),
    end_pos: (f32, f32)
) {
    println!("\nGenerating 2D Path Morph: ({:.1},{:.1}) -> ({:.1},{:.1}) in {}", 
        start_pos.0, start_pos.1, end_pos.0, end_pos.1, filename);
        
    let output_dir = "samples/morphing";
    let mut engine = OdinEngine::new(sample_rate);
    
    // Use the same arpeggio pattern as linear transition for better comparison
    let duration = 10.0;
    let note_duration = 0.125;
    let num_notes = (duration / note_duration) as usize;
    let notes = [60, 64, 67, 72, 67, 64]; // C Major Arpeggio
    
    let mut buffer = Vec::new();
    let samples_per_note = (sample_rate * note_duration) as usize;
    
    for i in 0..num_notes {
        let t = i as f32 / num_notes as f32;
        
        // Calculate current 2D position
        let x = start_pos.0 + (end_pos.0 - start_pos.0) * t;
        let y = start_pos.1 + (end_pos.1 - start_pos.1) * t;
        
        // --- THE KEY 2D MORPH CALL ---
        let morphed_preset = OdinPreset::morph_2d(sources, x, y);
        
        // Update Engine
        engine.load_preset(&morphed_preset).expect("Matrice de modulation du preset invalide");
        
        // Play note
        let note = notes[i % notes.len()];
        engine.note_on(note, 100);
        
        // Process Audio Block
        let mut note_buffer = vec![0.0f32; samples_per_note * 2];
        for j in 0..samples_per_note {
            let mut frame = [0.0; 2];
            engine.process(&mut frame, 2);
            note_buffer[j*2] = frame[0];
            note_buffer[j*2+1] = frame[1];
        }
        
        // Note off
        engine.note_off(note);
        buffer.extend_from_slice(&note_buffer);
    }
    
    let wav_path = format!("{}/{}", output_dir, filename);
    write_wav(&wav_path, &buffer, sample_rate as u32);
    println!("  ✓ Saved to {}", wav_path);
}

/// Generates a circular morph around the center (0,0)
fn generate_circle_morph(
    filename: &str,
    sample_rate: f32,
    sources: &[(OdinPreset, f32, f32)],
    radius: f32
) {
    println!("\nGenerating Circular Morph: Radius {} in {}", radius, filename);
    
    let output_dir = "samples/morphing";
    let mut engine = OdinEngine::new(sample_rate);
    
    // Arpeggio pattern
    let duration = 10.0;
    let note_duration = 0.125;
    let num_notes = (duration / note_duration) as usize;
    let notes = [60, 64, 67, 72, 67, 64]; // C Major Arpeggio
    
    let mut buffer = Vec::new();
    let samples_per_note = (sample_rate * note_duration) as usize;
    
    for i in 0..num_notes {
        let t = i as f32 / num_notes as f32; // 0.0 to 1.0
        
        // Calculate circular position
        // Start from -PI/2 (Bottom) or PI (Left)? Let's start at Top (0)
        // The x position should go from -1 (Left) to 1 (Right)
        // The y position should go from -1 (Bottom) to 1 (Top)
        
        // Let's do a full rotation starting from Top (0, 1) -> Right (1, 0) -> Bottom (0, -1) -> Left (-1, 0) -> Top
        // Angle goes from PI/2 down to -3*PI/2?
        // Or standard unit circle: 0 is Right (1,0), PI/2 is Top (0,1)
        
        let angle = t * 2.0 * std::f32::consts::PI; 
        
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        // --- THE KEY 2D MORPH CALL ---
        let morphed_preset = OdinPreset::morph_2d(sources, x, y);
        
        // Update Engine
        engine.load_preset(&morphed_preset).expect("Matrice de modulation du preset invalide");
        
        // Play note
        let note = notes[i % notes.len()];
        engine.note_on(note, 100);
        
        // Process Audio Block
        let mut note_buffer = vec![0.0f32; samples_per_note * 2];
        for j in 0..samples_per_note {
            let mut frame = [0.0; 2];
            engine.process(&mut frame, 2);
            note_buffer[j*2] = frame[0];
            note_buffer[j*2+1] = frame[1];
        }
        
        // Note off
        engine.note_off(note);
        buffer.extend_from_slice(&note_buffer);
    }
    
    let wav_path = format!("{}/{}", output_dir, filename);
    write_wav(&wav_path, &buffer, sample_rate as u32);
    println!("  ✓ Saved to {}", wav_path);
}

/// Generates a circular morph around the center (0,0) for Drums (Steady beat)
fn generate_drum_circle_morph(
    filename: &str,
    sample_rate: f32,
    sources: &[(OdinPreset, f32, f32)],
    radius: f32
) {
    println!("\nGenerating Circular Drum Morph: Radius {} in {}", radius, filename);
    
    let output_dir = "samples/morphing";
    let mut engine = OdinEngine::new(sample_rate);
    
    // Steady beat pattern (quarter notes)
    let duration = 10.0;
    
    // Slower temp for drums to hear detail
    let note_duration = 0.25; // Quarter note at 120bpm approx (0.5s) -> let's do 8th notes (0.25s)
    let num_notes = (duration / note_duration) as usize;
    
    // Just a fixed note C4 (60)
    let note = 60;
    
    let mut buffer = Vec::new();
    let samples_per_note = (sample_rate * note_duration) as usize;
    
    for i in 0..num_notes {
        let t = i as f32 / num_notes as f32; // 0.0 to 1.0
        
        // Full circle rotation
        let angle = t * 2.0 * std::f32::consts::PI; 
        
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        // --- THE KEY 2D MORPH CALL ---
        let morphed_preset = OdinPreset::morph_2d(sources, x, y);
        
        // Update Engine
        engine.load_preset(&morphed_preset).expect("Matrice de modulation du preset invalide");
        
        // Play note
        engine.note_on(note, 100);
        
        // Process Audio Block
        let mut note_buffer = vec![0.0f32; samples_per_note * 2];
        for j in 0..samples_per_note {
            let mut frame = [0.0; 2];
            engine.process(&mut frame, 2);
            note_buffer[j*2] = frame[0];
            note_buffer[j*2+1] = frame[1];
        }
        
        // Note off
        engine.note_off(note);
        buffer.extend_from_slice(&note_buffer);
    }
    
    let wav_path = format!("{}/{}", output_dir, filename);
    write_wav(&wav_path, &buffer, sample_rate as u32);
    println!("  ✓ Saved to {}", wav_path);
}

fn run_drum_morph_demo(sample_rate: f32) {
    let path_prefix = "odin2/assets/Soundbanks/Factory Presets/Drums";

    // Load Drum Presets
    let p1 = OdinPreset::load(format!("{}/Kick-1 [Photonic].odin", path_prefix))
        .expect("Failed to load Kick");
    let p2 = OdinPreset::load(format!("{}/Snare-1 [Photonic].odin", path_prefix))
        .expect("Failed to load Snare");
    // Use Drum Machine to test Sequencer morphing
    let p3 = OdinPreset::load(format!("{}/Drum Machine.odin", path_prefix))
        .expect("Failed to load Drum Machine");
    let p4 = OdinPreset::load(format!("{}/HiHat-closed [Photonic].odin", path_prefix))
        .expect("Failed to load HiHat");

    println!("✓ Loaded Drum Presets: Kick, Snare, Drum Machine, HiHat");

    let sources = vec![
        (p1, -1.0, 1.0),   // Top-Left: Kick
        (p2, 1.0, 1.0),    // Top-Right: Snare
        (p3, -1.0, -1.0),  // Bottom-Left: Drum Machine (Seq)
        (p4, 1.0, -1.0),   // Bottom-Right: HiHat
    ];

    // Generate previews to make sure they work
    generate_preview("preview_drum_kick.wav", sample_rate, &sources[0].0);
    generate_preview("preview_drum_snare.wav", sample_rate, &sources[1].0);
    generate_preview("preview_drum_machine.wav", sample_rate, &sources[2].0);
    generate_preview("preview_drum_hihat.wav", sample_rate, &sources[3].0);

    // Circular Morph for Drums (using repeated single note)
    generate_drum_circle_morph("morph_drums_circle.wav", sample_rate, &sources, 0.8);
    
    // Also try a path directly into the Drum Machine from Kick
    // Kick (No Arp) -> Drum Machine (Arp)
    generate_linear_transition("transition_kick_to_machine.wav", sample_rate, &sources[0].0, &sources[2].0);
}
