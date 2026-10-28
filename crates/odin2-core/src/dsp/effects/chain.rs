//! Chaîne FX exacte de `PluginProcessorProcess.cpp:441-472` (GPL-3.0-or-later).
//!
//! Ordre stocké comme les positions du patch: delay/phaser/flanger/chorus/reverb.
//! Le filtre 3 précède cette chaîne; le master la suit. La distorsion est par voix.
//! `new` réserve toute la mémoire; configure/process/modulation/tempo/reset n'allouent pas.

use crate::mod_matrix::ModulationValues;
use crate::preset::{sync_ratio, ChorusParams, DelayParams, FlangerParams, PhaserParams, ReverbParams};
use super::{Chorus, Delay, Flanger, Phaser, ZitaReverb};

#[derive(Clone, Copy)]
enum Effect { Delay, Phaser, Flanger, Chorus, Reverb }

pub struct EffectsChain {
    delay: Delay,
    phaser: Phaser,
    flanger: [Flanger; 2],
    chorus: Chorus,
    reverb: ZitaReverb,
    slots: [Option<Effect>; 5],
    delay_params: DelayParams,
    phaser_params: PhaserParams,
    flanger_params: FlangerParams,
    chorus_params: ChorusParams,
    reverb_params: ReverbParams,
    configured: bool,
    bpm: f32,
}
impl EffectsChain {
    /// Fréquence supportée: 8–192 kHz, comme les buffers fixes Odin 2.
    pub fn new(sample_rate: f32) -> Self {
        let mut right = Flanger::new(sample_rate);
        right.set_lfo_reset_pos(0.5);
        // PluginProcessorMisc.cpp:228 (initializeModules): le Phaser() amont utilise 1,32 mais le
        // processeur impose 1,25 avant tout rendu.
        let mut phaser = Phaser::new(sample_rate);
        phaser.set_radius(1.25);
        Self {
            delay: Delay::new(sample_rate), phaser,
            flanger: [Flanger::new(sample_rate), right], chorus: Chorus::new(sample_rate),
            reverb: ZitaReverb::new(sample_rate), slots: [None; 5],
            delay_params: DelayParams::default(), phaser_params: PhaserParams::default(),
            flanger_params: FlangerParams::default(), chorus_params: ChorusParams::default(),
            reverb_params: ReverbParams::default(), configured: false, bpm: 120.0,
        }
    }
    pub fn configure(&mut self, delay: &DelayParams, phaser: &PhaserParams,
        flanger: &FlangerParams, chorus: &ChorusParams, reverb: &ReverbParams,
        positions: [i32; 5]) {
        self.delay.set_feedback(delay.feedback);
        self.delay.set_dry(delay.dry);
        self.delay.set_wet(delay.wet);
        self.delay.set_highpass_freq(delay.hp);
        self.delay.set_ducking(delay.ducking);
        self.delay.set_ping_pong(delay.pingpong);
        self.phaser.set_base_freq(phaser.frequency);
        self.phaser.set_feedback(phaser.feedback);
        self.phaser.set_amount(phaser.mod_amount);
        self.phaser.set_dry_wet(phaser.dry_wet);
        for f in &mut self.flanger {
            f.set_amount(flanger.amount);
            f.set_feedback(flanger.feedback);
            f.set_dry_wet(flanger.dry_wet);
        }
        self.chorus.set_amount(chorus.amount);
        self.chorus.set_feedback(chorus.feedback);
        self.chorus.set_dry_wet(chorus.dry_wet);
        if !self.configured || self.reverb_params.delay != reverb.delay {
            // PluginProcessorValueChange.cpp:687-688: le patch est en millisecondes.
            self.reverb.set_delay(reverb.delay / 1000.0);
        }
        if !self.configured || self.reverb_params.mid_hall != reverb.mid_hall { self.reverb.set_rtmid(reverb.mid_hall); }
        if !self.configured || self.reverb_params.hf_damp != reverb.hf_damp { self.reverb.set_fdamp(reverb.hf_damp); }
        if !self.configured || self.reverb_params.eq_gain != reverb.eq_gain { self.reverb.set_eq_gain(reverb.eq_gain); }
        if !self.configured || self.reverb_params.eq_freq != reverb.eq_freq { self.reverb.set_eq_freq(reverb.eq_freq); }
        if !self.configured || self.reverb_params.dry_wet != reverb.dry_wet { self.reverb.set_mix(reverb.dry_wet); }
        let reset_delay = (!self.configured && delay.on) || self.delay_params.on != delay.on;
        // PluginProcessorValueChange.cpp:662-663,1152-1156: `delay_time` ne règle le temps que lorsqu'il
        // change (ou quand la synchro s'éteint); sinon le temps synchronisé posé par setBPM est conservé.
        let time_written = !self.configured || self.delay_params.time != delay.time
            || (self.delay_params.sync && !delay.sync);
        let reset_reverb = reverb.on && (!self.configured || !self.reverb_params.on);
        self.delay_params = *delay;
        self.phaser_params = *phaser;
        self.flanger_params = *flanger;
        self.chorus_params = *chorus;
        self.reverb_params = *reverb;
        // Le chargement règle `delay_time` (valeur libre) avant la remise à zéro (AudioValueTree.h:282,295);
        // le temps synchronisé n'arrive qu'avec setBPM au bloc suivant (PluginProcessorProcess.cpp:34): le
        // lissage du delay part donc de la valeur libre.
        if time_written { self.delay.set_delay_time(delay.time); }
        if reset_delay { self.delay.reset(); }
        self.update_rates();
        if reset_reverb { self.reverb.reset(); }
        for (slot, target) in self.slots.iter_mut().enumerate() {
            // La priorité else-if amont est conservée si un patch a des doublons.
            *target = if positions[0] == slot as i32 { Some(Effect::Delay) }
                else if positions[1] == slot as i32 { Some(Effect::Phaser) }
                else if positions[2] == slot as i32 { Some(Effect::Flanger) }
                else if positions[3] == slot as i32 { Some(Effect::Chorus) }
                else if positions[4] == slot as i32 { Some(Effect::Reverb) }
                else { None };
        }
        self.configured = true;
    }
    fn update_rates(&mut self) {
        let d = &self.delay_params;
        self.delay.set_delay_time(if d.sync { 240.0 * sync_ratio(d.sync_numerator, d.sync_denominator) / self.bpm } else { d.time });
        let p = &self.phaser_params;
        self.phaser.set_lfo_freq(if p.sync { self.bpm / sync_ratio(p.sync_numerator, p.sync_denominator) / 240.0 } else { p.rate });
        let f = &self.flanger_params;
        let frequency = if f.sync { self.bpm / sync_ratio(f.sync_numerator, f.sync_denominator) / 240.0 } else { f.rate };
        for flanger in &mut self.flanger { flanger.set_lfo_freq(frequency); }
        let c = &self.chorus_params;
        self.chorus.set_lfo_freq(if c.sync { self.bpm / sync_ratio(c.sync_numerator, c.sync_denominator) / 240.0 } else { c.rate });
    }
    pub fn set_bpm(&mut self, bpm: f32) {
        if self.bpm != bpm { self.bpm = bpm; self.update_rates(); }
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.delay.set_sample_rate(sample_rate);
        self.phaser.set_sample_rate(sample_rate);
        for f in &mut self.flanger { f.set_sample_rate(sample_rate); }
        self.chorus.set_sample_rate(sample_rate);
        self.reverb.set_sample_rate(sample_rate);
        self.update_rates();
    }
    pub fn set_modulation(&mut self, values: &ModulationValues) {
        self.delay.set_modulation(values.get(701), values.get(702), values.get(703), values.get(704), values.get(705));
        self.phaser.set_modulation(values.get(751), values.get(752), values.get(753), values.get(754), values.get(755));
        for f in &mut self.flanger { f.set_modulation(values.get(801), values.get(802), values.get(803), values.get(804)); }
        self.chorus.set_modulation(values.get(851), values.get(852), values.get(853), values.get(854));
    }
    /// PluginProcessorMidi.cpp:84-94: reset de phase, jamais des queues d'effet.
    pub fn note_on(&mut self) {
        if self.phaser_params.reset { self.phaser.reset_lfo(); }
        if self.flanger_params.reset { for f in &mut self.flanger { f.reset_lfo(); } }
        if self.chorus_params.reset { self.chorus.reset_lfo(); }
    }
    pub fn reset(&mut self) {
        self.delay.reset(); self.phaser.reset();
        for f in &mut self.flanger { f.reset(); }
        self.chorus.reset(); self.reverb.reset();
    }
    pub fn process(&mut self, mut left: f32, mut right: f32) -> (f32, f32) {
        for effect in self.slots {
            (left, right) = match effect {
                Some(Effect::Delay) if self.delay_params.on => self.delay.process(left, right),
                Some(Effect::Phaser) if self.phaser_params.on => self.phaser.process(left, right),
                Some(Effect::Flanger) if self.flanger_params.on => (self.flanger[0].process(left), self.flanger[1].process(right)),
                Some(Effect::Chorus) if self.chorus_params.on => self.chorus.process_stereo(left, right),
                Some(Effect::Reverb) if self.reverb_params.on => self.reverb.process(left, right),
                _ => (left, right),
            };
        }
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> (DelayParams, PhaserParams, FlangerParams, ChorusParams, ReverbParams) {
        (DelayParams { time: 0.01, dry: 1.0, hp: 80.0, ..Default::default() },
         PhaserParams { frequency: 4000.0, rate: 0.25, ..Default::default() },
         FlangerParams { rate: 0.2, ..Default::default() },
         ChorusParams { rate: 0.15, ..Default::default() },
         ReverbParams { delay: 40.0, mid_hall: 2.0, hf_damp: 3000.0, eq_freq: 1000.0, ..Default::default() })
    }
    #[test]
    fn disabled_chain_is_exact_stereo_bypass() {
        let mut chain = EffectsChain::new(44100.0);
        let (d,p,f,c,r) = parameters();
        chain.configure(&d,&p,&f,&c,&r,[0,1,3,2,4]);
        assert_eq!(chain.process(0.7,-0.4), (0.7,-0.4));
    }
    #[test]
    fn unrelated_configuration_preserves_delay_tail() {
        let mut chain = EffectsChain::new(44100.0);
        let (mut d,p,mut f,c,r) = parameters();
        d.on = true; d.dry = 0.0; d.wet = 1.0;
        chain.configure(&d,&p,&f,&c,&r,[0,1,3,2,4]);
        chain.process(1.0,0.0);
        f.feedback = -0.3;
        chain.configure(&d,&p,&f,&c,&r,[0,1,3,2,4]);
        let mut peak = 0.0_f32;
        for _ in 0..1000 { peak = peak.max(chain.process(0.0,0.0).0.abs()); }
        assert!(peak > 0.1);
    }
    #[test]
    fn duplicate_positions_follow_upstream_else_if_priority() {
        let mut chain = EffectsChain::new(44100.0);
        let (d,p,f,c,mut r) = parameters();
        r.on = true; r.dry_wet = 1.0;
        chain.configure(&d,&p,&f,&c,&r,[0,1,2,3,0]);
        assert_eq!(chain.process(0.7,-0.4), (0.7,-0.4));
    }
}
