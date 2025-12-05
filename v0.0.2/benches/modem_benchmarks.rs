// benches/modem_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use hsf_softmodem::dsp::fsk::*;
use hsf_softmodem::dsp::qam_modem::*;
use hsf_softmodem::dsp::qam::*;

fn bench_fsk_modulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fsk_modulation");
    
    let mode = FSKMode::Bell103Originate;
    let mut modulator = FSKModulator::new(mode, 300.0, 8000.0);
    let test_data = b"The quick brown fox jumps over the lazy dog";
    
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    group.bench_function("Bell 103", |b| {
        b.iter(|| {
            modulator.modulate_bytes(black_box(test_data))
        });
    });
    
    group.finish();
}

fn bench_qam_modulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("qam_modulation");
    
    let test_data = b"The quick brown fox jumps over the lazy dog";
    
    // V.22
    let mut mod_v22 = QAMModulator::new(QAMMode::V22, 1200.0, 8000.0);
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    group.bench_function("V.22 (1200 bps)", |b| {
        b.iter(|| {
            mod_v22.modulate_bytes(black_box(test_data))
        });
    });
    
    // V.22bis
    let mut mod_v22bis = QAMModulator::new(QAMMode::V22bis, 1200.0, 8000.0);
    group.bench_function("V.22bis (2400 bps)", |b| {
        b.iter(|| {
            mod_v22bis.modulate_bytes(black_box(test_data))
        });
    });
    
    group.finish();
}

fn bench_demodulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("demodulation");
    
    // Generate test audio
    let mode = FSKMode::Bell103Originate;
    let mut modulator = FSKModulator::new(mode, 300.0, 8000.0);
    let audio = modulator.modulate_bytes(b"TEST DATA");
    
    let mut demodulator = FSKDemodulator::new(mode, 300.0, 8000.0);
    group.throughput(Throughput::Elements(audio.len() as u64));
    group.bench_function("FSK Demod", |b| {
        b.iter(|| {
            demodulator.demodulate(black_box(&audio))
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_fsk_modulation, bench_qam_modulation, bench_demodulation);
criterion_main!(benches);
