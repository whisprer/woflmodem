// tests/dsp_tests.rs
use hsf_softmodem::dsp::*;
use hsf_softmodem::dsp::oscillator::*;
use hsf_softmodem::dsp::filters::*;
use hsf_softmodem::dsp::goertzel::*;
use hsf_softmodem::dsp::fsk::*;
use approx::assert_relative_eq;
use std::f32::consts::PI;

#[test]
fn test_nco_frequency_generation() {
    let mut nco = NCO::new(1000.0, 8000.0, 1.0);
    let mut samples = vec![0.0; 8];
    nco.generate(&mut samples);
    
    // At 1000 Hz with 8000 Hz sample rate, we complete 1 cycle in 8 samples
    // Verify we're generating a sine wave
    assert!(samples[0].abs() < 0.1);  // Start near zero
    assert!(samples[2] > 0.9);        // Peak near 1.0
    assert!(samples[6] < -0.9);       // Trough near -1.0
}

#[test]
fn test_biquad_lowpass_filter() {
    let mut filter = BiquadFilter::lowpass(1000.0, 0.707, 8000.0);
    
    // Test DC (0 Hz) - should pass
    let dc_signal = vec![1.0; 100];
    let mut output = vec![0.0; 100];
    filter.process_block(&dc_signal, &mut output);
    
    // After settling, DC should pass through
    assert!(output[50..].iter().all(|&x| x > 0.9));
}

#[test]
fn test_biquad_bandpass_filter() {
    let center_freq = 1200.0;
    let bandwidth = 400.0;
    let mut filter = BiquadFilter::bandpass(center_freq, bandwidth, 8000.0);
    
    // Generate signal at center frequency
    let mut nco = NCO::new(center_freq, 8000.0, 1.0);
    let mut input = vec![0.0; 200];
    nco.generate(&mut input);
    
    let mut output = vec![0.0; 200];
    filter.process_block(&input, &mut output);
    
    // Signal at center frequency should pass (after transient)
    let rms_output: f32 = output[100..].iter().map(|x| x * x).sum::<f32>() / 100.0;
    assert!(rms_output.sqrt() > 0.5);
}

#[test]
fn test_goertzel_tone_detection() {
    let target_freq = 1200.0;
    let sample_rate = 8000.0;
    let block_size = 100;
    
    let mut detector = GoertzelDetector::new(target_freq, sample_rate, block_size);
    
    // Generate tone at target frequency
    let mut nco = NCO::new(target_freq, sample_rate, 1.0);
    for _ in 0..block_size {
        detector.process_sample(nco.next());
    }
    
    let magnitude = detector.magnitude();
    
    // Should detect strong presence of target frequency
    assert!(magnitude > 40.0, "Magnitude: {}", magnitude);
}

#[test]
fn test_goertzel_rejects_other_frequencies() {
    let target_freq = 1200.0;
    let wrong_freq = 2400.0;
    let sample_rate = 8000.0;
    let block_size = 100;
    
    let mut detector = GoertzelDetector::new(target_freq, sample_rate, block_size);
    
    // Generate tone at different frequency
    let mut nco = NCO::new(wrong_freq, sample_rate, 1.0);
    for _ in 0..block_size {
        detector.process_sample(nco.next());
    }
    
    let magnitude = detector.magnitude();
    
    // Should not detect wrong frequency strongly
    assert!(magnitude < 10.0, "Magnitude: {}", magnitude);
}

#[test]
fn test_dual_tone_detector() {
    let mark_freq = 1270.0;
    let space_freq = 1070.0;
    let sample_rate = 8000.0;
    let block_size = 100;
    
    let mut detector = DualToneDetector::new(mark_freq, space_freq, sample_rate, block_size);
    
    // Test with mark frequency (should return true)
    let mut nco = NCO::new(mark_freq, sample_rate, 1.0);
    for _ in 0..block_size {
        detector.process_sample(nco.next());
    }
    
    assert!(detector.detect_bit());
    detector.reset();
    
    // Test with space frequency (should return false)
    nco.set_frequency(space_freq, sample_rate);
    for _ in 0..block_size {
        detector.process_sample(nco.next());
    }
    
    assert!(!detector.detect_bit());
}

#[test]
fn test_dtmf_generator() {
    let mut gen = DTMFGenerator::new(8000.0);
    
    // Generate '5' (770 Hz + 1336 Hz)
    let tone = gen.generate_digit('5', 800);
    
    assert_eq!(tone.len(), 800);
    
    // Verify it's not silence
    let rms: f32 = tone.iter().map(|x| x * x).sum::<f32>() / tone.len() as f32;
    assert!(rms.sqrt() > 0.3);
}

#[test]
fn test_fsk_loopback_bell103() {
    let mode = FSKMode::Bell103Originate;
    let baud_rate = 300.0;
    let sample_rate = 9600.0;
    
    let mut modulator = FSKModulator::new(mode, baud_rate, sample_rate);
    let mut demodulator = FSKDemodulator::new(mode, baud_rate, sample_rate);
    
    // Test data
    let test_bits = vec![true, false, true, true, false, false, true, false];
    
    // Modulate
    let audio = modulator.modulate(&test_bits);
    
    // Demodulate
    let received_bits = demodulator.demodulate(&audio);
    
    // Verify (may have some startup transients, check majority)
    let matching = test_bits.iter()
        .zip(received_bits.iter())
        .filter(|(&a, &b)| a == b)
        .count();
    
    let accuracy = matching as f32 / test_bits.len() as f32;
    assert!(accuracy > 0.85, "Accuracy: {:.2}%", accuracy * 100.0);
}

#[test]
fn test_fsk_byte_loopback() {
    let mode = FSKMode::Bell103Originate;
    let baud_rate = 300.0;
    let sample_rate = 9600.0;
    
    let mut modulator = FSKModulator::new(mode, baud_rate, sample_rate);
    let mut demodulator = FSKDemodulator::new(mode, baud_rate, sample_rate);
    
    // Test ASCII string
    let test_data = b"HELLO WORLD!";
    
    // Modulate
    let audio = modulator.modulate_bytes(test_data);
    
    // Demodulate
    let received_data = demodulator.demodulate_bytes(&audio);
    
    // Compare (skip first few bytes for sync)
    let start_idx = received_data.len().saturating_sub(test_data.len());
    let received_slice = &received_data[start_idx..];
    
    assert_eq!(test_data, received_slice, 
        "Expected: {:?}, Got: {:?}", test_data, received_slice);
}
