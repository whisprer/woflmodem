// src/lib.rs - Export modules for testing
pub mod audio;
pub mod dsp;
pub mod tapi;

// Re-export commonly used items
pub use dsp::SAMPLE_RATE;