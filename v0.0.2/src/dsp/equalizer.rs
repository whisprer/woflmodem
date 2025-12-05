// src/dsp/equalizer.rs
// Warning Fix: Removed unused 'use super::*;'
use num_complex::Complex32;

/// LMS adaptive equalizer [web:90][web:91]
pub struct LMSEqualizer {
// ... (Rest of the file is unchanged)
    taps: Vec<Complex32>,
    num_taps: usize,
    step_size: f32,  // μ (mu) parameter
    buffer: Vec<Complex32>,
    buffer_idx: usize,
    training: bool,
}

impl LMSEqualizer {
    /// Create new LMS equalizer [web:91]
    pub fn new(num_taps: usize, step_size: f32) -> Self {
        let mut taps = vec![Complex32::new(0.0, 0.0); num_taps];
        taps[num_taps / 2] = Complex32::new(1.0, 0.0);  // Center tap = 1
        
        Self {
            taps,
            num_taps,
            step_size,
            buffer: vec![Complex32::new(0.0, 0.0); num_taps],
            buffer_idx: 0,
            training: true,
        }
    }
    
    /// Process symbol and update taps
    pub fn equalize(
        &mut self, 
        input: Complex32, 
        training_symbol: Option<Complex32>
    ) -> Complex32 {
        // 1. Shift input buffer and insert new symbol
        self.buffer[self.buffer_idx] = input;
        self.buffer_idx = (self.buffer_idx + 1) % self.num_taps;
        
        // 2. Compute output: y(n) = W^T * X
        let mut output = Complex32::new(0.0, 0.0);
        for i in 0..self.num_taps {
            // Read from buffer (reverse order for convolution)
            let buffer_pos = (self.buffer_idx + i) % self.num_taps;
            output += self.taps[i] * self.buffer[buffer_pos];
        }
        
        // 3. Compute error and update taps
        if let Some(desired) = training_symbol {
            // Training mode: known symbol
            let error = desired - output;
            self.update_taps(error);
        } else if !self.training {
            // Decision-directed mode: use sliced output
            let decided = self.slice_symbol(output);
            let error = decided - output;
            self.update_taps(error);
        }
        
        output
    }
    
    /// LMS weight update: w(n+1) = w(n) + μ * e(n) * conj(x(n)) [web:90][web:91]
    fn update_taps(&mut self, error: Complex32) {
        for i in 0..self.num_taps {
            // Get the corresponding input from the buffer (reverse order)
            let buffer_pos = (self.buffer_idx + i) % self.num_taps;
            let input_conj = self.buffer[buffer_pos].conj();
            // w(n+1) = w(n) + μ * e(n) * x*(n)
            self.taps[i] += Complex32::new(self.step_size, 0.0) * error * input_conj;
        }
    }
    
    /// Simple hard decision (override for specific constellation)
    fn slice_symbol(&self, symbol: Complex32) -> Complex32 {
        // Round to nearest integer I and Q
        Complex32::new(symbol.re.round(), symbol.im.round())
    }
    
    pub fn set_training_mode(&mut self, training: bool) {
        self.training = training;
    }

    pub fn reset(&mut self) {
        self.buffer = vec![Complex32::new(0.0, 0.0); self.num_taps];
        self.buffer_idx = 0;
        self.taps = vec![Complex32::new(0.0, 0.0); self.num_taps];
        self.taps[self.num_taps / 2] = Complex32::new(1.0, 0.0);
        self.training = true;
    }
}