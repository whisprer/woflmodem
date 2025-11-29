// src/dsp/filters.rs
use super::*;

/// Second-order IIR filter (Direct Form II)
#[derive(Clone)]
pub struct BiquadFilter {
    // Feed-forward coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    
    // Feedback coefficients
    a1: f32,
    a2: f32,
    
    // State variables
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    /// Create bandpass filter
    pub fn bandpass(center_freq: f32, bandwidth: f32, sample_rate: f32) -> Self {
        let omega = freq_to_omega(center_freq, sample_rate);
        let bw = freq_to_omega(bandwidth, sample_rate);
        
        let alpha = (bw / 2.0).sin();
        let cos_omega = omega.cos();
        
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    
    /// Create lowpass filter
    pub fn lowpass(cutoff_freq: f32, q: f32, sample_rate: f32) -> Self {
        let omega = freq_to_omega(cutoff_freq, sample_rate);
        let alpha = omega.sin() / (2.0 * q);
        let cos_omega = omega.cos();
        
        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = b0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    
    /// Process single sample
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
    
    /// Process block of samples
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        for (i, o) in input.iter().zip(output.iter_mut()) {
            *o = self.process(*i);
        }
    }
    
    /// Reset filter state
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}
