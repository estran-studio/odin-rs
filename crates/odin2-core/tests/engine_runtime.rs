//! Régressions des contrats audio, sans accès aux presets d'une machine donnée.

use odin2_core::{EngineConfig, OdinEngine, ParameterId, SynthEngine};
use odin2_core::preset::{FilterType, OscillatorType};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct AudioAllocator;
std::thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static OPERATIONS: Cell<usize> = const { Cell::new(0) };
}
fn record() {
    let _ = TRACK.try_with(|track| {
        if track.get() { let _ = OPERATIONS.try_with(|count| count.set(count.get() + 1)); }
    });
}
unsafe impl GlobalAlloc for AudioAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { record(); System.alloc(layout) }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) { record(); System.dealloc(pointer, layout); }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(); System.realloc(pointer, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: AudioAllocator = AudioAllocator;

fn patch() -> EngineConfig {
    let mut config = EngineConfig::default();
    config.osc1.osc_type = OscillatorType::Analog;
    config.osc1.reset = true;
    config.osc2.osc_type = OscillatorType::None;
    config.osc3.osc_type = OscillatorType::None;
    config.filter1.filter_type = FilterType::None;
    config.filter1.to_amp = true;
    config.filter2.to_amp = false;
    config.env1.attack = 0.001;
    config.env1.decay = 0.001;
    config.env1.sustain = 1.0;
    config.env1.release = 0.005;
    config
}

#[test]
fn midi_render_and_live_parameters_do_not_touch_allocator() {
    let mut engine = OdinEngine::new(48000.0);
    engine.load_config(patch()).unwrap();
    let cutoff = ParameterId::from_name("fil1_freq").unwrap();
    let mix = ParameterId::from_name("delay_wet").unwrap();
    let mut audio = [0.0; 512];
    OPERATIONS.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    for note in 48..80 {
        engine.note_on(note, 100);
        engine.process(&mut audio, 2);
        engine.set_parameter(cutoff, 700.0 + f32::from(note)).unwrap();
        engine.set_parameter(mix, 0.25).unwrap();
        engine.note_off(note);
    }
    engine.control_change(64, 127);
    engine.note_on(60, 100);
    engine.note_off(60);
    engine.control_change(64, 0);
    engine.pitch_bend(4096);
    engine.process(&mut audio, 2);
    TRACK.with(|track| track.set(false));
    let operations = OPERATIONS.with(Cell::get);
    assert_eq!(operations, 0, "allocation/libération dans le chemin audio/MIDI");
    assert!(audio.iter().all(|sample| sample.is_finite()));
}

#[test]
fn render_is_deterministic_and_independent_of_block_size() {
    let mut a = OdinEngine::new(48000.0);
    let mut b = OdinEngine::new(48000.0);
    a.load_config(patch()).unwrap();
    b.load_config(patch()).unwrap();
    a.note_on(48, 100);
    b.note_on(48, 100);
    let mut whole = [0.0; 4096];
    let mut fragmented = [0.0; 4096];
    a.process(&mut whole, 2);
    for block in fragmented.chunks_mut(74) { b.process(block, 2); }
    assert_eq!(whole, fragmented);
    let energy: f32 = whole.iter().map(|sample| sample * sample).sum();
    assert!(energy > 0.01, "le scénario doit réellement rendre la note");
}

#[test]
fn prepared_engine_moves_to_audio_thread_without_changing_render() {
    let mut reference = OdinEngine::new(48000.0);
    let mut transferred = OdinEngine::new(48000.0);
    reference.load_config(patch()).unwrap();
    transferred.load_config(patch()).unwrap();
    reference.note_on(48, 100);
    let mut expected = [0.0; 4096];
    reference.process(&mut expected, 2);

    // Le chargement reste sur le fil principal ; seule l'instance prête traverse.
    let actual = std::thread::spawn(move || {
        let mut output = [0.0; 4096];
        OPERATIONS.with(|count| count.set(0));
        TRACK.with(|track| track.set(true));
        transferred.note_on(48, 100);
        transferred.process(&mut output, 2);
        transferred.note_off(48);
        TRACK.with(|track| track.set(false));
        assert_eq!(OPERATIONS.with(Cell::get), 0);
        output
    }).join().unwrap();

    assert_eq!(actual, expected);
    assert!(actual.iter().map(|sample| sample * sample).sum::<f32>() > 0.01);
}

#[test]
fn note_off_releases_every_repeated_note_and_unison_voice() {
    let mut config = patch();
    config.unison_voices = 4;
    let mut engine = OdinEngine::new(48000.0);
    engine.load_config(config).unwrap();
    engine.note_on(60, 100);
    engine.note_on(60, 100);
    assert_eq!(engine.active_voice_count(), 8);
    engine.control_change(64, 127);
    engine.note_off(60);
    let mut block = [0.0; 512];
    for _ in 0..40 { engine.process(&mut block, 2); }
    assert_eq!(engine.active_voice_count(), 8, "sustain garde les deux groupes d'unisson");
    engine.control_change(64, 0);
    for _ in 0..100 { engine.process(&mut block, 2); }
    assert_eq!(engine.active_voice_count(), 0, "aucune voix ne doit rester bloquée");
}

#[test]
fn live_volume_parameter_changes_an_already_sounding_note() {
    let mut engine = OdinEngine::new(48000.0);
    engine.load_config(patch()).unwrap();
    engine.note_on(48, 100);
    let mut block = [0.0; 4096];
    engine.process(&mut block, 2);
    let before: f32 = block.iter().map(|x| x * x).sum();
    let volume = ParameterId::from_name("osc1_vol").unwrap();
    engine.set_parameter(volume, -60.0).unwrap();
    for _ in 0..5 { engine.process(&mut block, 2); }
    let after: f32 = block.iter().map(|x| x * x).sum();
    assert!(after < before * 0.0001, "paramètre hôte ignoré sur la voix active : {before} -> {after}");
}
