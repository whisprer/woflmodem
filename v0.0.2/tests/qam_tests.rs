// tests/ber_tests.rs
use hsf_softmodem::dsp::fsk::*;
use hsf_softmodem::dsp::qam_modem::*;
use hsf_softmodem::dsp::qam::*;
use rand::Rng;

/// Add AWGN noise to signal [web:106][web:109]
fn add_awgn(signal: &[f32], snr_db: f32) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    
    // Calculate signal power
    let signal_power: f32 = signal.iter().map(|x| x * x).sum::<f32>() / signal.len() as f32;
    
    // Calculate noise power from SNR
    let snr_linear = 10f32.powf(snr_db / 10.0);
    let noise_power = signal_power / snr_linear;
    let noise_std = noise_power.sqrt();
    
    // Add Gaussian noise
    signal.iter()
        .map(|&s| s + rng.gen::<f32>() * noise_std * 2.0 - noise_std)
        .collect()
}

#[test]
fn test_fsk_ber_clean_channel() {
    let mode = FSKMode::Bell103Originate;
    let mut modulator = FSKModulator::new(mode, 300.0, 8000.0);
    let mut demodulator = FSKDemodulator::new(mode, 300.0, 8000.0);
    
    // Generate 1000 random bits
    let mut rng = rand::thread_rng();
    let test_bits: Vec<bool> = (0..1000).map(|_| rng.gen()).collect();
    
    // Modulate
    let audio = modulator.modulate(&test_bits);
    
    // Demodulate
    let received = demodulator.demodulate(&audio);
    
    // Calculate BER [web:100][web:101]
    let errors = test_bits.iter()
        .zip(received.iter())
        .filter(|(&a, &b)| a != b)
        .count();
    
    let ber = errors as f32 / test_bits.len() as f32;
    
    println!("FSK BER (clean channel): {:.6} ({} errors / {} bits)", 
        ber, errors, test_bits.len());
    
    assert!(ber < 0.01, "BER too high: {}", ber);  // < 1% [web:101]
}

#[test]
fn test_fsk_ber_with_noise() {
    let snr_values = vec![20.0, 15.0, 10.0, 5.0];
    
    for snr_db in snr_values {
        let mode = FSKMode::Bell103Originate;
        let mut modulator = FSKModulator::new(mode, 300.0, 8000.0);
        let mut demodulator = FSKDemodulator::new(mode, 300.0, 8000.0);
        
        // Generate test data
        let mut rng = rand::thread_rng();
        let test_bits: Vec<bool> = (0..500).map(|_| rng.gen()).collect();
        
        // Modulate
        let audio = modulator.modulate(&test_bits);
        
        // Add noise
        let noisy_audio = add_awgn(&audio, snr_db);
        
        // Demodulate
        let received = demodulator.demodulate(&noisy_audio);
        
        // Calculate BER
        let errors = test_bits.iter()
            .zip(received.iter())
            .filter(|(&a, &b)| a != b)
            .count();
        
        let ber = errors as f32 / test_bits.len().min(received.len()) as f32;
        
        println!("FSK BER at SNR {} dB: {:.6}", snr_db, ber);
    }
}

#[test]
fn test_qam_ber_performance() {
    let modes = vec![
        (QAMMode::V22, "V.22 (1200 bps DPSK)"),
        (QAMMode::V22bis, "V.22bis (2400 bps QAM)"),
    ];
    
    for (mode, name) in modes {
        println!("\n=== {} ===", name);
        
        let snr_values = vec![25.0, 20.0, 15.0];
        
        for snr_db in snr_values {
            let mut modulator = QAMModulator::new(mode, 1200.0, 8000.0);
            let mut demodulator = QAMDemodulator::new(mode, 1200.0, 8000.0);
            
            // Generate test bytes
            let mut rng = rand::thread_rng();
            let test_data: Vec<u8> = (0..100).map(|_| rng.gen()).collect();
            
            // Modulate
            let audio = modulator.modulate_bytes(&test_data);
            
            // Add preamble and noise
            let mut padded = vec![0.0; 1000];
            padded.extend(&audio);
            let noisy = add_awgn(&padded, snr_db);
            
            // Demodulate
            let received = demodulator.demodulate_bytes(&noisy);
            
            // Calculate byte error rate
            let byte_errors = if received.len() >= test_data.len() {
                test_data.iter()
                    .zip(received.iter().skip(received.len() - test_data.len()))
                    .filter(|(&a, &b)| a != b)
                    .count()
            } else {
                test_data.len()
            };
            
            let ber = byte_errors as f32 / test_data.len() as f32;
            
            println!("  SNR {} dB: BER = {:.6} ({} errors)", snr_db, ber, byte_errors);
        }
    }
}
