// src/dsp/goertzel.rs
use super::*;

/// Efficient single-frequency DFT using Goertzel algorithm
pub struct GoertzelDetector {
    coefficient: f32,
    s1: f32,
    s2: f32,
    n: usize,
    block_size: usize,
}

impl GoertzelDetector {
    /// Create detector for specific frequency [web:54][web:55]
    pub fn new(target_freq: f32, sample_rate: f32, block_size: usize) -> Self {
        let k = (0.5 + (block_size as f32 * target_freq / sample_rate)) as usize;
        let omega = (2.0 * PI * k as f32) / block_size as f32;
        let coefficient = 2.0 * omega.cos();
        
        Self {
            coefficient,
            s1: 0.0,
            s2: 0.0,
            n: 0,
            block_size,
        }
    }
    
    /// Process single sample [web:53]
    #[inline]
    pub fn process_sample(&mut self, sample: f32) {
        let s0 = sample + self.coefficient * self.s1 - self.s2;
        self.s2 = self.s1;
        self.s1 = s0;
        self.n += 1;
    }
    
    /// Get magnitude squared (energy) at target frequency
    pub fn magnitude_squared(&self) -> f32 {
        self.s1 * self.s1 + self.s2 * self.s2 - self.coefficient * self.s1 * self.s2
    }
    
    /// Get magnitude
    pub fn magnitude(&self) -> f32 {
        self.magnitude_squared().sqrt()
    }
    
    /// Reset detector
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
        self.n = 0;
    }
    
    /// Check if block is complete
    pub fn is_complete(&self) -> bool {
        self.n >= self.block_size
    }
}

/// Dual-tone detector for FSK demodulation
pub struct DualToneDetector {
    detector_mark: GoertzelDetector,
    detector_space: GoertzelDetector,
}

impl DualToneDetector {
    pub fn new(
        mark_freq: f32,
        space_freq: f32,
        sample_rate: f32,
        block_size: usize,
    ) -> Self {
        Self {
            detector_mark: GoertzelDetector::new(mark_freq, sample_rate, block_size),
            detector_space: GoertzelDetector::new(space_freq, sample_rate, block_size),
        }
    }
    
    /// Process sample through both detectors [web:41][web:42]
    pub fn process_sample(&mut self, sample: f32) {
        self.detector_mark.process_sample(sample);
        self.detector_space.process_sample(sample);
    }
    
    /// Determine which tone is stronger (true = mark/1, false = space/0)
    pub fn detect_bit(&self) -> bool {
        let mark_energy = self.detector_mark.magnitude_squared();
        let space_energy = self.detector_space.magnitude_squared();
        mark_energy > space_energy
    }
    
    /// Get energy ratio (mark / space)
    pub fn energy_ratio(&self) -> f32 {
        let mark = self.detector_mark.magnitude_squared();
        let space = self.detector_space.magnitude_squared();
        if space > 1e-10 {
            mark / space
        } else {
            0.0
        }
    }
    
    /// Reset both detectors
    pub fn reset(&mut self) {
        self.detector_mark.reset();
        self.detector_space.reset();
    }
    
    pub fn is_complete(&self) -> bool {
        self.detector_mark.is_complete()
    }
}
