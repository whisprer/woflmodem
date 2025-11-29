// src/dsp/scrambler.rs
// Warning Fix: Removed unused 'use super::*;'

/// V.22bis scrambler: qi = di ⊕ qi-14 ⊕ qi-17 [web:95]
pub struct Scrambler {
// ... (Rest of the file is unchanged)
    shift_register: u32,  // 17-bit shift register
}

impl Scrambler {
    pub fn new() -> Self {
        Self {
            shift_register: 0,
        }
    }
    
    /// Scramble single bit [web:95][web:98]
    pub fn scramble_bit(&mut self, input_bit: bool) -> bool {
        // V.22bis taps: 14 and 17. The register is 17 bits (0 to 16).
        // bit14 is at index 13 (17-14=3 bits from LSB, or bit 14, i.e., 2^13)
        // bit17 is at index 16 (17-17=0 bits from LSB, or bit 17, i.e., 2^16)
        let bit14 = (self.shift_register >> 13) & 1;  // qi-14
        let bit17 = (self.shift_register >> 16) & 1;  // qi-17
        
        let output_bit = (input_bit as u32) ^ bit14 ^ bit17;
        
        // Shift register and insert new bit (up to 17 bits max: 0x1FFFF)
        self.shift_register = ((self.shift_register << 1) | output_bit) & 0x1FFFF;
        
        output_bit != 0
    }
    
    /// Scramble byte (LSB first)
    pub fn scramble_byte(&mut self, byte: u8) -> u8 {
        let mut result = 0u8;
        for i in 0..8 {
            let input_bit = (byte >> i) & 1;
            let scrambled_bit = self.scramble_bit(input_bit != 0) as u8;
            result |= scrambled_bit << i;
        }
        result
    }

    pub fn reset(&mut self) {
        self.shift_register = 0;
    }
}