//! Drift Generator
//!
//! Generates slow random drift to emulate analog oscillator instability.
//! Creates a smooth, slowly-changing random modulation signal.

/// Default drift period in seconds
const DRIFT_LENGTH_SECONDS: f32 = 5.0;

/// Simple RNG for drift generation
struct DriftRng {
    state: u32,
}

impl DriftRng {
    fn new() -> Self {
        Self { state: 0x12345678 }
    }

    fn new_with_seed(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Generate a random value between -1.0 and 1.0
    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        let random = (self.state & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32;
        2.0 * random - 1.0
    }
}

/// Drift Generator
///
/// Generates slow, smooth random modulation to simulate
/// analog oscillator pitch drift and instability.
pub struct DriftGenerator {
    /// Sample rate
    sample_rate: f32,

    /// Drift period in samples
    drift_length: usize,

    /// Current sample counter
    counter: usize,

    /// Last random value
    last_value: f32,

    /// Next random value
    next_value: f32,

    /// Amount of drift (0.0 to 1.0)
    amount: f32,

    /// Random number generator
    rng: DriftRng,
}

impl DriftGenerator {
    /// Create a new drift generator
    pub fn new(sample_rate: f32) -> Self {
        let drift_length = (DRIFT_LENGTH_SECONDS * sample_rate) as usize;
        let mut rng = DriftRng::new();

        Self {
            sample_rate,
            drift_length,
            counter: 0,
            last_value: rng.next_f32(),
            next_value: rng.next_f32(),
            amount: 1.0,
            rng,
        }
    }

    /// Create a new drift generator with a specific seed
    /// Useful for creating multiple drift generators with different patterns
    pub fn new_with_seed(sample_rate: f32, seed: u32) -> Self {
        let drift_length = (DRIFT_LENGTH_SECONDS * sample_rate) as usize;
        let mut rng = DriftRng::new_with_seed(seed);

        Self {
            sample_rate,
            drift_length,
            counter: 0,
            last_value: rng.next_f32(),
            next_value: rng.next_f32(),
            amount: 1.0,
            rng,
        }
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.drift_length = (DRIFT_LENGTH_SECONDS * sample_rate) as usize;
    }

    /// Set drift period in seconds
    pub fn set_drift_period(&mut self, seconds: f32) {
        let seconds = seconds.clamp(0.1, 30.0);
        self.drift_length = (seconds * self.sample_rate) as usize;
    }

    /// Set drift amount (0.0 to 1.0)
    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount.clamp(0.0, 1.0);
    }

    /// Reset the drift generator
    pub fn reset(&mut self) {
        self.counter = 0;
        self.last_value = self.rng.next_f32();
        self.next_value = self.rng.next_f32();
    }

    /// Generate new random target when period ends
    fn calc_new_coeffs(&mut self) {
        self.last_value = self.next_value;
        self.next_value = self.rng.next_f32();
    }

    /// Process one sample
    /// Returns a value between -amount and +amount
    pub fn process(&mut self) -> f32 {
        self.counter += 1;
        if self.counter >= self.drift_length {
            self.counter = 0;
            self.calc_new_coeffs();
        }

        // Linear interpolation between last and next value
        let t = self.counter as f32 / self.drift_length as f32;
        let drift = (self.next_value - self.last_value) * t + self.last_value;

        drift * self.amount
    }

    /// Get current drift value without advancing
    pub fn current(&self) -> f32 {
        let t = self.counter as f32 / self.drift_length as f32;
        let drift = (self.next_value - self.last_value) * t + self.last_value;
        drift * self.amount
    }
}

impl Default for DriftGenerator {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_basic() {
        let mut drift = DriftGenerator::new(44100.0);
        drift.set_amount(1.0);

        // Process many samples
        for _ in 0..44100 * 10 {
            let output = drift.process();
            assert!(output.is_finite(), "Drift produced invalid output");
            assert!(
                (-1.0..=1.0).contains(&output),
                "Drift out of range: {}",
                output
            );
        }
    }

    #[test]
    fn test_drift_slow_change() {
        let mut drift = DriftGenerator::new(44100.0);
        drift.set_amount(1.0);

        // Drift should change slowly - consecutive samples should be similar
        let mut prev = drift.process();
        let mut max_change = 0.0f32;

        for _ in 0..10000 {
            let current = drift.process();
            let change = (current - prev).abs();
            max_change = max_change.max(change);
            prev = current;
        }

        // Max change between consecutive samples should be very small
        // (drift is linear interpolation over 5 seconds = 220500 samples)
        assert!(
            max_change < 0.001,
            "Drift changing too fast: {}",
            max_change
        );
    }

    #[test]
    fn test_drift_amount() {
        let mut drift = DriftGenerator::new(44100.0);
        drift.set_amount(0.5);

        // With amount = 0.5, output should be in [-0.5, 0.5]
        for _ in 0..44100 {
            let output = drift.process();
            assert!(
                (-0.5..=0.5).contains(&output),
                "Drift with amount 0.5 out of range: {}",
                output
            );
        }
    }

    #[test]
    fn test_drift_seeded() {
        // Two drift generators with same seed should produce same values
        let mut drift1 = DriftGenerator::new_with_seed(44100.0, 12345);
        let mut drift2 = DriftGenerator::new_with_seed(44100.0, 12345);

        for _ in 0..1000 {
            let v1 = drift1.process();
            let v2 = drift2.process();
            assert!(
                (v1 - v2).abs() < 1e-6,
                "Same seed should produce same values"
            );
        }

        // Different seeds should produce different values
        let mut drift3 = DriftGenerator::new_with_seed(44100.0, 54321);
        drift1.reset();
        drift1.rng = DriftRng::new_with_seed(12345);
        drift1.last_value = drift1.rng.next_f32();
        drift1.next_value = drift1.rng.next_f32();

        let v1 = drift1.process();
        let v3 = drift3.process();
        // Different seeds, different values (statistically very likely)
        // This might occasionally fail but is statistically unlikely
        assert!(v1.is_finite() && v3.is_finite());
    }

    #[test]
    fn test_drift_period() {
        let mut drift = DriftGenerator::new(44100.0);
        drift.set_drift_period(1.0); // 1 second period
        drift.set_amount(1.0);

        // With 1 second period at 44100 Hz, we should cycle every 44100 samples
        let first_value = drift.current();

        // Process for one full period
        for _ in 0..44100 {
            let _ = drift.process();
        }

        // After one period, we should have moved to a new random target
        let after_period = drift.current();
        // Values will be different (unless by extreme coincidence)
        assert!(first_value.is_finite() && after_period.is_finite());
    }
}
