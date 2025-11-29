// src/audio/mod.rs

pub mod ringbuffer;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use crossbeam_channel::{bounded, Receiver, Sender};

use ringbuffer::RingBuffer;

#[derive(Debug, Clone, Copy)]
pub struct ModemAudioConfig {
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub channels: u16,
    pub buffer_duration_ms: u32,
}

impl Default for ModemAudioConfig {
    fn default() -> Self {
        Self {
            // You can tie this to crate::dsp::SAMPLE_RATE later if you like.
            sample_rate: 8000,
            bits_per_sample: 16,
            channels: 1,
            buffer_duration_ms: 20,
        }
    }
}

#[derive(Debug)]
pub enum AudioCommand {
    Start,
    Stop,
    SendSamples(Vec<f32>),
    Capture,
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    CapturedSamples(Vec<f32>),
    PlaybackReady,
    Error(String),
}

///
/// A small, lock-free, command-driven audio engine.
///
/// Right now this is a simulation layer that uses a ring buffer to move
/// samples between the "playback" side and the "capture" side.  The public
/// API is intentionally WASAPI-ish so that we can later swap in a real
/// device backend without touching higher-level modem code.
///
pub struct WasapiAudioEngine {
    config: ModemAudioConfig,
    running: Arc<AtomicBool>,
    cmd_tx: Sender<AudioCommand>,
    cmd_rx: Receiver<AudioCommand>,
    event_tx: Sender<AudioEvent>,
    event_rx: Receiver<AudioEvent>,
    playback_buffer: Arc<RingBuffer<f32>>,
}

impl WasapiAudioEngine {
    pub fn new(config: ModemAudioConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let (cmd_tx, cmd_rx) = bounded(64);
        let (event_tx, event_rx) = bounded(256);

        // Allow a few buffers worth of audio to queue up.
        let frames_per_buffer =
            (config.sample_rate as u64 * config.buffer_duration_ms as u64 / 1000) as usize;
        let capacity = frames_per_buffer * config.channels as usize * 4;

        let playback_buffer = Arc::new(RingBuffer::new(capacity.max(1)));

        Ok(Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            cmd_tx,
            cmd_rx,
            event_tx,
            event_rx,
            playback_buffer,
        })
    }

    /// For the simulated backend there's nothing to initialize yet, but we keep
    /// the hook so API stays compatible with a real WASAPI implementation.
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.swap(true, Ordering::SeqCst) {
            // already running
            return Ok(());
        }

        let running = self.running.clone();
        let cmd_rx = self.cmd_rx.clone();
        let event_tx = self.event_tx.clone();
        let playback_buffer = self.playback_buffer.clone();
        let cfg = self.config;

        thread::Builder::new()
            .name("woflmodem-audio".to_string())
            .spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match cmd_rx.recv() {
                        Ok(AudioCommand::Start) => {
                            // nothing special yet – kept for future expansion
                        }
                        Ok(AudioCommand::Stop) => {
                            running.store(false, Ordering::SeqCst);
                        }
                        Ok(AudioCommand::SendSamples(samples)) => {
                            let written = playback_buffer.write(&samples);
                            if written < samples.len() {
                                let _ = event_tx.send(AudioEvent::Error(
                                    "Playback ring buffer overflow".to_string(),
                                ));
                            } else {
                                let _ = event_tx.send(AudioEvent::PlaybackReady);
                            }
                        }
                        Ok(AudioCommand::Capture) => {
                            // Read one nominal buffer worth of samples from the ring.
                            let frames = (cfg.sample_rate as u64
                                * cfg.buffer_duration_ms as u64
                                / 1000) as usize;
                            let mut buf =
                                vec![0.0f32; frames.saturating_mul(cfg.channels as usize).max(1)];
                            let read = playback_buffer.read(&mut buf);
                            buf.truncate(read);
                            let _ = event_tx.send(AudioEvent::CapturedSamples(buf));
                        }
                        Err(_) => {
                            running.store(false, Ordering::SeqCst);
                        }
                    }
                }
            })?;

        // Kick the worker
        let _ = self.cmd_tx.send(AudioCommand::Start);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.cmd_tx.send(AudioCommand::Stop);
    }

    /// Queue samples to be "played".  In the simulated backend this just writes
    /// into the ring buffer that `Capture` will later read from.
    pub fn queue_playback(&self, samples: Vec<f32>) {
        let _ = self.cmd_tx.send(AudioCommand::SendSamples(samples));
    }

    /// Request one capture buffer worth of samples from the engine.
    pub fn request_capture(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Capture);
    }

    /// Drain any pending events.
    pub fn poll_events(&self) -> Vec<AudioEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            events.push(ev);
        }
        events
    }
}
