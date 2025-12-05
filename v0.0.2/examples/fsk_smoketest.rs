// src/main.rs - DSP test
mod audio;
mod dsp;

use dsp::fsk::*;
use dsp::SAMPLE_RATE;

fn main() {
    env_logger::init();
    
    // Test FSK modulation/demodulation
    let mode = FSKMode::Bell103Originate;
    let baud_rate = 300.0;
    
    let mut modulator = FSKModulator::new(mode, baud_rate, SAMPLE_RATE);
    let mut demodulator = FSKDemodulator::new(mode, baud_rate, SAMPLE_RATE);
    
    // Test data: "HELLO"
    let test_data = b"HELLO";
    println!("Original: {:?}", test_data);
    
    // Modulate to audio
    let audio = modulator.modulate_bytes(test_data);
    println!("Generated {} audio samples", audio.len());
    
    // Demodulate back
    let received = demodulator.demodulate_bytes(&audio);
    println!("Received: {:?}", received);
    
    // Verify
    if test_data == &received[..] {
        println!("✓ FSK modem working perfectly!");
    } else {
        println!("✗ Decoding mismatch");
    }
}
