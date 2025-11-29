// src/dsp/fsk.rs
use super::*;
use super::oscillator::NCO;
use super::goertzel::DualToneDetector;
use super::filters::BiquadFilter;

/// Bell 103 / V.21 frequency specifications [web:46][web:49]
#[derive(Debug, Clone, Copy)]
pub enum FSKMode {
    Bell103Originate,  // 1070 Hz space, 1270 Hz mark
    Bell103Answer,     // 2025 Hz space, 2225 Hz mark
    V21Originate,      // 1180 Hz space, 980 Hz mark
    V21Answer,         // 1850 Hz space, 1650 Hz mark
}

impl FSKMode {
    pub fn frequencies(&self) -> (f32, f32) {
        match self {
            FSKMode::Bell103Originate => (1070.0, 1270.0),
            FSKMode::Bell103Answer => (2025.0, 2225.0),
            FSKMode::V21Originate => (1180.0, 980.0),
            FSKMode::V21Answer => (1850.0, 1650.0),
        }
    }
    
    pub fn space_freq(&self) -> f32 {
        self.frequencies().0
    }
    
    pub fn mark_freq(&self) -> f32 {
        self.frequencies().1
    }
    
    pub fn center_freq(&self) -> f32 {
        let (space, mark) = self.frequencies();
        (space + mark) / 2.0
    }
}

/// FSK modulator - converts bits to audio tones
pub struct FSKModulator {
    mode: FSKMode,
    osc: NCO,
    samples_per_bit: usize,
    sample_counter: usize,
}

impl FSKModulator {
    pub fn new(mode: FSKMode, baud_rate: f32, sample_rate: f32) -> Self {
        let samples_per_bit = (sample_rate / baud_rate) as usize;
        let (space_freq, _) = mode.frequencies();
        
        Self {
            mode,
            osc: NCO::new(space_freq, sample_rate, 1.0),
            samples_per_bit,
            sample_counter: 0,
        }
    }
    
    /// Modulate bits to audio samples
    pub fn modulate(&mut self, bits: &[bool]) -> Vec<f32> {
        let mut samples = Vec::with_capacity(bits.len() * self.samples_per_bit);
        
        for &bit in bits {
            let freq = if bit {
                self.mode.mark_freq()
            } else {
                self.mode.space_freq()
            };
            
            self.osc.set_frequency(freq, SAMPLE_RATE);
            
            for _ in 0..self.samples_per_bit {
                samples.push(self.osc.next());
            }
        }
        
        samples
    }
    
    /// Modulate bytes to audio
    pub fn modulate_bytes(&mut self, data: &[u8]) -> Vec<f32> {
        let mut bits = Vec::with_capacity(data.len() * 8);
        for &byte in data {
            for i in 0..8 {
                bits.push((byte >> i) & 1 == 1);
            }
        }
        self.modulate(&bits)
    }
}

/// FSK demodulator - converts audio tones to bits
pub struct FSKDemodulator {
    mode: FSKMode,
    bandpass: BiquadFilter,
    detector: DualToneDetector,
    samples_per_bit: usize,
    sample_buffer: Vec<f32>,
    bit_buffer: Vec<bool>,
}

impl FSKDemodulator {
    pub fn new(mode: FSKMode, baud_rate: f32, sample_rate: f32) -> Self {
        let samples_per_bit = (sample_rate / baud_rate) as usize;
        let (space_freq, mark_freq) = mode.frequencies();
        let center_freq = mode.center_freq();
        let bandwidth = (mark_freq - space_freq).abs() * 2.0;
        
        Self {
            mode,
            bandpass: BiquadFilter::bandpass(center_freq, bandwidth, sample_rate),
            detector: DualToneDetector::new(mark_freq, space_freq, sample_rate, samples_per_bit),
            samples_per_bit,
            sample_buffer: Vec::new(),
            bit_buffer: Vec::new(),
        }
    }
    
    /// Process audio samples and extract bits
    pub fn demodulate(&mut self, samples: &[f32]) -> Vec<bool> {
        self.bit_buffer.clear();
        
        for &sample in samples {
            // Bandpass filter
            let filtered = self.bandpass.process(sample);
            
            // Feed to Goertzel detectors
            self.detector.process_sample(filtered);
            
            // Check if we have a complete bit period
            if self.detector.is_complete() {
                let bit = self.detector.detect_bit();
                self.bit_buffer.push(bit);
                self.detector.reset();
            }
        }
        
        self.bit_buffer.clone()
    }
    
    /// Demodulate to bytes (LSB first)
    pub fn demodulate_bytes(&mut self, samples: &[f32]) -> Vec<u8> {
        let bits = self.demodulate(samples);
        let mut bytes = Vec::new();
        
        for chunk in bits.chunks(8) {
            if chunk.len() == 8 {
                let mut byte = 0u8;
                for (i, &bit) in chunk.iter().enumerate() {
                    if bit {
                        byte |= 1 << i;
                    }
                }
                bytes.push(byte);
            }
        }
        
        bytes
    }
}
