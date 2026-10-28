//! Allocation d'Odin 2, traduite de `Source/audio/Voice.h:638-825`.
//! Historique et listes de sustain bornés, sans conteneur alloué en MIDI/audio.

use crate::constants::VOICES;

pub(crate) const MAX_UNISON: usize = 12;

pub(crate) struct VoiceManager {
    pub history: [usize; VOICES],
    pub sustain: bool,
    pub deferred: [bool; VOICES],
    deferred_note: [u8; VOICES],
}

impl VoiceManager {
    pub fn new() -> Self {
        Self {
            history: core::array::from_fn(|i| i),
            sustain: false,
            deferred: [false; VOICES],
            deferred_note: [0; VOICES],
        }
    }

    pub fn defer(&mut self, voice: usize, note: u8) {
        self.deferred[voice] = true;
        self.deferred_note[voice] = note;
    }

    fn touch(&mut self, voice: usize) {
        let index = self.history.iter().position(|&v| v == voice).unwrap_or(0);
        self.history.copy_within(0..index, 1);
        self.history[0] = voice;
        self.deferred[voice] = false;
    }

    pub fn allocate(
        &mut self,
        note: u8,
        count: usize,
        poly: bool,
        active: &[bool; VOICES],
        releasing: &[bool; VOICES],
    ) -> [usize; MAX_UNISON] {
        let mut result = [0; MAX_UNISON];
        if !poly {
            for (i, voice) in result.iter_mut().enumerate().take(count) {
                *voice = i;
                self.deferred[i] = false;
            }
            return result;
        }
        let mut used = [false; VOICES];
        let mut found = 0;
        // Priorité 1 : même touche maintenue par la pédale.
        if self.sustain {
            for voice in 0..VOICES {
                if self.deferred[voice] && self.deferred_note[voice] == note {
                    self.deferred[voice] = false;
                    result[found] = voice;
                    used[voice] = true;
                    found += 1;
                    if found == count { return result; }
                }
            }
        }
        // Priorité 2 : indices libres ascendants.
        for voice in 0..VOICES {
            if !active[voice] && !used[voice] {
                result[found] = voice;
                used[voice] = true;
                self.touch(voice);
                found += 1;
                if found == count { return result; }
            }
        }
        // Priorité 3 : release le plus ancien, puis voix la plus ancienne.
        for release_only in [true, false] {
            while found < count {
                let candidate = self.history.iter().rev().copied().find(|&v| {
                    !used[v] && (!release_only || releasing[v])
                });
                let Some(voice) = candidate else { break };
                result[found] = voice;
                used[voice] = true;
                self.touch(voice);
                found += 1;
            }
        }
        result
    }
}

/// Distributions de `PluginProcessor.h:313-331`, y compris l'ordre de désaccordage.
pub(crate) fn unison(count: usize, index: usize) -> (f32, f32, f32) {
    const PAN4: [f32; 4] = [-1.0, -0.33333, 0.3333333, 1.0];
    const PAN6: [f32; 6] = [-1.0, -0.6, -0.2, 0.2, 0.6, 1.0];
    const PAN12: [f32; 12] = [-1.0, -0.8181, -0.6363, -0.45454, -0.2787,
        -0.0909, 0.0909, 0.2787, 0.45454, 0.6363, 0.8181, 1.0];
    match count {
        2 => { let p = [-1.0, 1.0][index]; (p, p, 0.86) }
        3 => { let p = [-1.0, 0.0, 1.0][index]; (p, p, 0.75) }
        4 => (PAN4[index], PAN4[[2, 0, 3, 1][index]], 0.65),
        6 => (PAN6[index], PAN6[[1, 4, 2, 0, 5, 3][index]], 0.53),
        12 => (PAN12[index], PAN12[[3, 9, 6, 10, 7, 4, 0, 11, 5, 1, 8, 2][index]], 0.45),
        _ => (0.0, 0.0, 1.0),
    }
}

pub(crate) fn unison_count(count: i32) -> usize {
    match count { 2 | 3 | 4 | 6 | 12 => count as usize, _ => 1 }
}
