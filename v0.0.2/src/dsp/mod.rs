// src/dsp/mod.rs
pub mod oscillator;
pub mod filters;
pub mod goertzel;
pub mod fsk;
pub mod timing;
pub mod carrier;
pub mod qam;
pub mod qam_modem;
pub mod scrambler;
pub mod equalizer;
pub mod costas;

use std::f32::consts::PI;

/// Standard telephone line sample rate
pub const SAMPLE_RATE: f32 = 8000.0;

#[inline]
pub fn freq_to_omega(freq_hz: f32, sample_rate: f32) -> f32 {
    2.0 * PI * freq_hz / sample_rate
}
