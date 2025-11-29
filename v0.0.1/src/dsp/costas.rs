// src/dsp/costas.rs
use super::*;
use num_complex::Complex32;

/// Costas loop for carrier recovery [web:86][web:89]
pub struct CostasLoop {
    phase: f32,
    frequency: f32,
    
    // Loop filter (PI controller)
    proportional_gain: f32,
    integral_gain: f32,
    integrator: f32,
    
    // NCO
    carrier_freq: f32,
    sample_rate: f32,
}

impl CostasLoop {
    /// Create Costas loop [web:86]
    pub fn new(
        carrier_freq: f32, 
        sample_rate: f32, 
        loop_bandwidth: f32
    ) -> Self {
        // Design loop filter coefficients
        let damping = 0.707;  // Critical damping
        let theta = loop_bandwidth / sample_rate;
        let denom = 1.0 + 2.0 * damping * theta + theta * theta;
        
        let proportional_gain = (4.0 * damping * theta) / denom;
        let integral_gain = (4.0 * theta * theta) / denom;
        
        Self {
            phase: 0.0,
            frequency: carrier_freq,
            proportional_gain,
            integral_gain,
            integrator: 0.0,
            carrier_freq,
            sample_rate,
        }
    }
    
    /// Demodulate sample and track carrier [web:86][web:89]
    pub fn process(&mut self, input: f32) -> Complex32 {
        let omega = freq_to_omega(self.frequency, self.sample_rate);
        
        // Generate quadrature carriers
        let i_carrier = self.phase.cos();
        let q_carrier = self.phase.sin();
        
        // Demodulate (mix down to baseband)
        let i_signal = input * i_carrier;
        let q_signal = input * q_carrier;
        
        // Phase error detector (for QPSK/QAM)
        // Simple decision-directed: sign(I)*Q - sign(Q)*I
        let phase_error = i_signal.signum() * q_signal - q_signal.signum() * i_signal;
        
        // Loop filter (PI controller)
        self.integrator += phase_error * self.integral_gain;
        let freq_correction = phase_error * self.proportional_gain + self.integrator;
        
        // Update frequency and phase
        self.frequency = self.carrier_freq + freq_correction * self.sample_rate / (2.0 * PI);
        self.phase += omega + freq_correction;
        
        // Wrap phase
        while self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
        while self.phase < 0.0 {
            self.phase += 2.0 * PI;
        }
        
        Complex32::new(i_signal, q_signal)
    }
    
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.frequency = self.carrier_freq;
        self.integrator = 0.0;
    }
    
    pub fn is_locked(&self) -> bool {
        // Simple lock detector: frequency error within tolerance
        (self.frequency - self.carrier_freq).abs() < 10.0
    }
}
