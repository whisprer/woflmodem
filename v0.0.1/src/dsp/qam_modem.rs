// src/dsp/qam_modem.rs
use super::*;
use super::qam::*;
use super::scrambler::Scrambler;
use super::equalizer::LMSEqualizer;
use super::costas::CostasLoop;
use super::oscillator::NCO;
use super::filters::BiquadFilter;
use num_complex::Complex32;

/// QAM/DPSK modulator for V.22/V.22bis [web:82][web:84]
pub struct QAMModulator {
    mode: QAMMode,
    carrier_freq: f32,
    sample_rate: f32,
    symbol_rate: f32,
    samples_per_symbol: usize,
    
    // Signal generation
    i_osc: NCO,
    q_osc: NCO,
    scrambler: Scrambler,
    
    // DPSK state
    dpsk_phase: f32,
    
    // Pulse shaping filter
    tx_filter: BiquadFilter,
}

impl QAMModulator {
    pub fn new(
        mode: QAMMode,
        carrier_freq: f32,
        sample_rate: f32,
    ) -> Self {
        let symbol_rate = mode.symbol_rate();
        let samples_per_symbol = (sample_rate / symbol_rate) as usize;
        
        // Quadrature oscillators (90° phase shift)
        let i_osc = NCO::new(carrier_freq, sample_rate, 1.0);
        let q_osc = NCO::new(carrier_freq, sample_rate, 1.0);
        
        // Transmit filter (root raised cosine approximation with lowpass)
        let cutoff = symbol_rate * 0.6;
        let tx_filter = BiquadFilter::lowpass(cutoff, 0.707, sample_rate);
        
        Self {
            mode,
            carrier_freq,
            sample_rate,
            symbol_rate,
            samples_per_symbol,
            i_osc,
            q_osc,
            scrambler: Scrambler::new(),
            dpsk_phase: 0.0,
            tx_filter,
        }
    }
    
    /// Modulate bits to audio samples
    pub fn modulate(&mut self, bits: &[bool]) -> Vec<f32> {
        let bits_per_symbol = self.mode.bits_per_symbol();
        let mut samples = Vec::new();
        
        // Process symbols
        for symbol_bits in bits.chunks(bits_per_symbol) {
            if symbol_bits.len() != bits_per_symbol {
                break;  // Incomplete symbol
            }
            
            // Pack bits into byte
            let mut symbol_data = 0u8;
            for (i, &bit) in symbol_bits.iter().enumerate() {
                if bit {
                    symbol_data |= 1 << i;
                }
            }
            
            // Scramble
            let scrambled = if bits_per_symbol == 2 {
                let mut s = 0u8;
                s |= if self.scrambler.scramble_bit((symbol_data >> 0) & 1 != 0) { 1 } else { 0 };
                s |= if self.scrambler.scramble_bit((symbol_data >> 1) & 1 != 0) { 2 } else { 0 };
                s
            } else {
                self.scrambler.scramble_byte(symbol_data)
            };
            
            // Map to constellation
            let baseband_symbol = match self.mode {
                QAMMode::V22 | QAMMode::Bell212A => {
                    // DPSK modulation [web:96]
                    let phase_shift = map_to_dpsk(scrambled);
                    self.dpsk_phase += phase_shift;
                    while self.dpsk_phase >= 2.0 * PI {
                        self.dpsk_phase -= 2.0 * PI;
                    }
                    Complex32::new(self.dpsk_phase.cos(), self.dpsk_phase.sin())
                }
                QAMMode::V22bis => {
                    // 16-QAM [web:82][web:88]
                    map_to_qam16(scrambled)
                }
            };
            
            // Generate samples for this symbol
            for _ in 0..self.samples_per_symbol {
                let i_carrier = self.i_osc.next();
                let q_carrier = self.q_osc.next();
                
                // Quadrature modulation
                let sample = baseband_symbol.re * i_carrier - baseband_symbol.im * q_carrier;
                
                // Pulse shaping
                let filtered = self.tx_filter.process(sample);
                samples.push(filtered);
            }
        }
        
        samples
    }
    
    /// Modulate bytes
    pub fn modulate_bytes(&mut self, data: &[u8]) -> Vec<f32> {
        let mut bits = Vec::with_capacity(data.len() * 8);
        for &byte in data {
            for i in 0..8 {
                bits.push((byte >> i) & 1 != 0);
            }
        }
        self.modulate(&bits)
    }
    
    pub fn reset(&mut self) {
        self.scrambler.reset();
        self.dpsk_phase = 0.0;
        self.tx_filter.reset();
    }
}

/// QAM/DPSK demodulator [web:84]
pub struct QAMDemodulator {
    mode: QAMMode,
    carrier_freq: f32,
    sample_rate: f32,
    symbol_rate: f32,
    samples_per_symbol: usize,
    
    // Carrier recovery
    costas: CostasLoop,
    
    // Equalization
    equalizer: LMSEqualizer,
    
    // Matched filter
    rx_filter: BiquadFilter,
    
    // Symbol timing
    sample_counter: f32,
    
    // Descrambling
    descrambler: Scrambler,
    
    // DPSK state
    prev_phase: f32,
    
    // Buffers
    symbol_buffer: Vec<Complex32>,
}

impl QAMDemodulator {
    pub fn new(
        mode: QAMMode,
        carrier_freq: f32,
        sample_rate: f32,
    ) -> Self {
        let symbol_rate = mode.symbol_rate();
        let samples_per_symbol = (sample_rate / symbol_rate) as usize;
        
        // Matched filter
        let cutoff = symbol_rate * 0.6;
        let rx_filter = BiquadFilter::lowpass(cutoff, 0.707, sample_rate);
        
        // Carrier recovery with Costas loop
        let loop_bw = symbol_rate * 0.05;  // 5% of symbol rate
        let costas = CostasLoop::new(carrier_freq, sample_rate, loop_bw);
        
        // Adaptive equalizer (17 taps typical for V.22bis)
        let equalizer = LMSEqualizer::new(17, 0.01);
        
        Self {
            mode,
            carrier_freq,
            sample_rate,
            symbol_rate,
            samples_per_symbol,
            costas,
            equalizer,
            rx_filter,
            sample_counter: 0.0,
            descrambler: Scrambler::new(),
            prev_phase: 0.0,
            symbol_buffer: Vec::new(),
        }
    }
    
    /// Demodulate audio samples to bits
    pub fn demodulate(&mut self, samples: &[f32]) -> Vec<bool> {
        let mut bits = Vec::new();
        
        for &sample in samples {
            // Matched filter
            let filtered = self.rx_filter.process(sample);
            
            // Carrier recovery (Costas loop)
            let baseband = self.costas.process(filtered);
            
            // Symbol timing
            self.sample_counter += 1.0;
            
            if self.sample_counter >= self.samples_per_symbol as f32 {
                self.sample_counter -= self.samples_per_symbol as f32;
                
                // Equalize symbol
                let equalized = self.equalizer.equalize(baseband, None);
                
                // Demodulate symbol
                let symbol_bits = self.demodulate_symbol(equalized);
                
                // Descramble
                let descrambled_bits = self.descramble_symbol(symbol_bits);
                bits.extend(descrambled_bits);
            }
        }
        
        bits
    }
    
    /// Demodulate single symbol
    fn demodulate_symbol(&mut self, symbol: Complex32) -> Vec<bool> {
        match self.mode {
            QAMMode::V22 | QAMMode::Bell212A => {
                // DPSK demodulation [web:96][web:99]
                let current_phase = symbol.im.atan2(symbol.re);
                let mut phase_diff = current_phase - self.prev_phase;
                
                // Wrap phase difference to [0, 2π)
                while phase_diff < 0.0 {
                    phase_diff += 2.0 * PI;
                }
                while phase_diff >= 2.0 * PI {
                    phase_diff -= 2.0 * PI;
                }
                
                self.prev_phase = current_phase;
                
                // Map phase to dibits
                let dibit = if phase_diff < PI / 4.0 || phase_diff >= 7.0 * PI / 4.0 {
                    0b00
                } else if phase_diff < 3.0 * PI / 4.0 {
                    0b01
                } else if phase_diff < 5.0 * PI / 4.0 {
                    0b10
                } else {
                    0b11
                };
                
                vec![(dibit & 1) != 0, (dibit & 2) != 0]
            }
            QAMMode::V22bis => {
                // 16-QAM slicing [web:88]
                let quadbits = slice_qam16(symbol);
                vec![
                    (quadbits & 1) != 0,
                    (quadbits & 2) != 0,
                    (quadbits & 4) != 0,
                    (quadbits & 8) != 0,
                ]
            }
        }
    }
    
    /// Descramble symbol bits
    fn descramble_symbol(&mut self, bits: Vec<bool>) -> Vec<bool> {
        bits.into_iter()
            .map(|b| self.descrambler.scramble_bit(b))
            .collect()
    }
    
    /// Demodulate to bytes
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
    
    pub fn reset(&mut self) {
        self.costas.reset();
        self.equalizer.reset();
        self.rx_filter.reset();
        self.descrambler.reset();
        self.sample_counter = 0.0;
        self.prev_phase = 0.0;
    }
    
    pub fn is_locked(&self) -> bool {
        self.costas.is_locked()
    }
}
