// src/dsp/oscillator.rs
use super::*;

/// Phase-accumulating sine wave generator
pub struct NCO {
    phase: f32,
    phase_increment: f32,
    amplitude: f32,
}

impl NCO {
    pub fn new(frequency: f32, sample_rate: f32, amplitude: f32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: freq_to_omega(frequency, sample_rate),
            amplitude,
        }
    }
    
    /// Set new frequency
    pub fn set_frequency(&mut self, frequency: f32, sample_rate: f32) {
        self.phase_increment = freq_to_omega(frequency, sample_rate);
    }
    
    /// Generate next sample
    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = self.amplitude * self.phase.sin();
        self.phase += self.phase_increment;
        
        // Wrap phase to prevent accumulation errors
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
        
        sample
    }
    
    /// Generate block of samples
    pub fn generate(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.next();
        }
    }
}

/// DTMF tone generator (for dialing)
pub struct DTMFGenerator {
    row_osc: NCO,
    col_osc: NCO,
}

impl DTMFGenerator {
    // DTMF frequency matrix
    const ROW_FREQS: [f32; 4] = [697.0, 770.0, 852.0, 941.0];
    const COL_FREQS: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];
    
    pub fn new(sample_rate: f32) -> Self {
        Self {
            row_osc: NCO::new(Self::ROW_FREQS[0], sample_rate, 0.5),
            col_osc: NCO::new(Self::COL_FREQS[0], sample_rate, 0.5),
        }
    }
    
    /// Generate DTMF tone for digit (0-9, *, #, A-D)
    pub fn generate_digit(&mut self, digit: char, duration_samples: usize) -> Vec<f32> {
        let (row_idx, col_idx) = match digit {
            '1' => (0, 0), '2' => (0, 1), '3' => (0, 2), 'A' => (0, 3),
            '4' => (1, 0), '5' => (1, 1), '6' => (1, 2), 'B' => (1, 3),
            '7' => (2, 0), '8' => (2, 1), '9' => (2, 2), 'C' => (2, 3),
            '*' => (3, 0), '0' => (3, 1), '#' => (3, 2), 'D' => (3, 3),
            _ => return vec![0.0; duration_samples],
        };
        
        self.row_osc.set_frequency(Self::ROW_FREQS[row_idx], SAMPLE_RATE);
        self.col_osc.set_frequency(Self::COL_FREQS[col_idx], SAMPLE_RATE);
        
        let mut samples = vec![0.0; duration_samples];
        for sample in samples.iter_mut() {
            *sample = self.row_osc.next() + self.col_osc.next();
        }
        
        samples
    }
}
