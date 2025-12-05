// src/dsp/carrier.rs
// Warning Fix: Removed unused 'use super::*;'
use super::goertzel::GoertzelDetector;

/// Detects presence of carrier signal
pub struct CarrierDetector {
// ... (Rest of the file is unchanged)
    detector: GoertzelDetector,
    energy_threshold: f32,
    present: bool,
}

impl CarrierDetector {
    pub fn new(
        carrier_freq: f32,
        sample_rate: f32,
        block_size: usize,
        threshold_db: f32,
    ) -> Self {
        Self {
            detector: GoertzelDetector::new(carrier_freq, sample_rate, block_size),
            // Convert dB threshold to linear power
            energy_threshold: 10f32.powf(threshold_db / 10.0), 
            present: false,
        }
    }
    
    pub fn process_sample(&mut self, sample: f32) {
        self.detector.process_sample(sample);
        
        if self.detector.is_complete() {
            let energy = self.detector.magnitude_squared();
            self.present = energy > self.energy_threshold;
            self.detector.reset();
        }
    }
    
    pub fn is_present(&self) -> bool {
        self.present
    }
    
    pub fn reset(&mut self) {
        self.detector.reset();
        self.present = false;
    }
}