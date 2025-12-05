// src/dsp/qam.rs
use super::*;
use num_complex::Complex32;

/// V.22/V.22bis mode specifications [web:82][web:84]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QAMMode {
    V22,         // 1200 bps DPSK, 600 baud, 2 bits/symbol
    V22bis,      // 2400 bps QAM, 600 baud, 4 bits/symbol
    Bell212A,    // 1200 bps DPSK (compatible with V.22)
}

impl QAMMode {
    pub fn symbol_rate(&self) -> f32 {
        600.0  // All modes use 600 baud
    }
    
    pub fn bits_per_symbol(&self) -> usize {
        match self {
            QAMMode::V22 | QAMMode::Bell212A => 2,
            QAMMode::V22bis => 4,
        }
    }
    
    pub fn data_rate(&self) -> u32 {
        match self {
            QAMMode::V22 | QAMMode::Bell212A => 1200,
            QAMMode::V22bis => 2400,
        }
    }
    
    pub fn carrier_freq_originate(&self) -> f32 {
        1200.0  // Low band
    }
    
    pub fn carrier_freq_answer(&self) -> f32 {
        2400.0  // High band
    }
}

/// 16-QAM constellation for V.22bis [web:82][web:88]
const QAM16_CONSTELLATION: [(i8, i8); 16] = [
    // (I, Q) amplitudes
    (1, 1),   (1, 3),   (3, 1),   (3, 3),    // Quadrant 1
    (-1, 1),  (-1, 3),  (-3, 1),  (-3, 3),   // Quadrant 2
    (-1, -1), (-1, -3), (-3, -1), (-3, -3),  // Quadrant 3
    (1, -1),  (1, -3),  (3, -1),  (3, -3),   // Quadrant 4
];

/// Map 4 bits to 16-QAM constellation point [web:82]
pub fn map_to_qam16(bits: u8) -> Complex32 {
    let idx = (bits & 0x0F) as usize;
    let (i, q) = QAM16_CONSTELLATION[idx];
    Complex32::new(i as f32, q as f32) / 3.0  // Normalize
}

/// Find nearest constellation point (slicer/decision)
pub fn slice_qam16(symbol: Complex32) -> u8 {
    let mut min_dist = f32::MAX;
    let mut best_idx = 0u8;
    
    for (idx, &(i, q)) in QAM16_CONSTELLATION.iter().enumerate() {
        let constellation_point = Complex32::new(i as f32, q as f32) / 3.0;
        let dist = (symbol - constellation_point).norm_sqr();
        if dist < min_dist {
            min_dist = dist;
            best_idx = idx as u8;
        }
    }
    
    best_idx
}

/// DPSK phase mapping for V.22 [web:96][web:99]
const DPSK_PHASE_MAP: [f32; 4] = [
    0.0,           // 00: 0°
    PI / 2.0,      // 01: 90°
    PI,            // 10: 180°
    3.0 * PI / 2.0 // 11: 270°
];

/// Map 2 bits to DPSK phase shift [web:96]
pub fn map_to_dpsk(bits: u8) -> f32 {
    DPSK_PHASE_MAP[(bits & 0x03) as usize]
}
