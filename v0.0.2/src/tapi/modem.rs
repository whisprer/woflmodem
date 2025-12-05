// src/tapi/modem.rs
//
// Virtual soft modem core: AT parser + FSK/QAM DSP + audio glue.

use super::at_commands::{ATCommand, ATCommandParser, ATResponse, ModemState};
use crate::audio::{AudioEvent, ModemAudioConfig, WasapiAudioEngine};
use crate::dsp::fsk::{FSKDemodulator, FSKMode, FSKModulator};
use crate::dsp::qam::QAMMode;
use crate::dsp::qam_modem::{QAMDemodulator, QAMModulator};
use crate::dsp::SAMPLE_RATE;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ESCAPE_SEQUENCE: &str = "+++";
const ESCAPE_GUARD_TIME: Duration = Duration::from_secs(1);
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModemMode {
    Bell103,
    V22,
    V22bis,
    Bell212A,
}

pub struct VirtualModem {
    state: Arc<Mutex<ModemState>>,
    pub parser: ATCommandParser,
    s_registers: [u8; 256],

    // Audio/DSP components
    audio_engine: Option<WasapiAudioEngine>,
    modulator: FSKModulator,
    demodulator: FSKDemodulator,

    // QAM (V.22/V.22bis/Bell212A)
    qam_modulator: Option<QAMModulator>,
    qam_demodulator: Option<QAMDemodulator>,
    current_mode: ModemMode,
    connection_speed: u32,

    // Connection / escape tracking
    connected: bool,
    escape_sequence_time: Option<Instant>,
    plus_count: u8,

    // Host-side data buffers
    tx_buffer: Vec<u8>,
    rx_buffer: Vec<u8>,
}

impl VirtualModem {
    fn default_s_registers() -> [u8; 256] {
        let mut regs = [0u8; 256];

        // Reasonable defaults.
        regs[3] = 13; // S3 – command line termination (CR)
        regs[4] = 10; // S4 – response line feed (LF)
        regs[5] = 8;  // S5 – backspace

        // Guard time for "+++": S12 is in 20 ms units: 50 × 20 ms = 1 s
        regs[12] = 50;

        regs
    }

    fn new_internal() -> Self {
        let state = Arc::new(Mutex::new(ModemState::Command));

        // Start in Bell 103 originate, 300 baud, using global SAMPLE_RATE.
        let fsk_mode = FSKMode::Bell103Originate;
        let baud_rate = 300.0_f32;
        let sample_rate = SAMPLE_RATE;

        let modulator = FSKModulator::new(fsk_mode, baud_rate, sample_rate);
        let demodulator = FSKDemodulator::new(fsk_mode, baud_rate, sample_rate);

        // Prepare QAM chain with a sensible default (V.22bis originate carrier).
        let qam_mode = QAMMode::V22bis;
        let carrier = qam_mode.carrier_freq_originate();
        let qam_modulator = Some(QAMModulator::new(qam_mode, carrier, sample_rate));
        let qam_demodulator = Some(QAMDemodulator::new(qam_mode, carrier, sample_rate));

        Self {
            state,
            parser: ATCommandParser::new(),
            s_registers: Self::default_s_registers(),
            audio_engine: None,
            modulator,
            demodulator,
            qam_modulator,
            qam_demodulator,
            current_mode: ModemMode::Bell103,
            connection_speed: 300,
            connected: false,
            escape_sequence_time: None,
            plus_count: 0,
            tx_buffer: Vec::new(),
            rx_buffer: Vec::new(),
        }
    }

    fn reconfigure_for_speed(&mut self, speed: u32) {
        self.connection_speed = speed;
        self.current_mode = match speed {
            300 => ModemMode::Bell103,
            1200 => ModemMode::V22,
            2400 => ModemMode::V22bis,
            _ => self.current_mode,
        };

        let baud_rate = speed as f32;
        let sample_rate = SAMPLE_RATE;

        // FSK chain for 300 baud and generic high-speed fallback.
        let fsk_mode = match self.current_mode {
            ModemMode::Bell103 => FSKMode::Bell103Originate,
            _ => FSKMode::V21Originate,
        };
        self.modulator = FSKModulator::new(fsk_mode, baud_rate, sample_rate);
        self.demodulator = FSKDemodulator::new(fsk_mode, baud_rate, sample_rate);

        // QAM for V-series modes; Bell103 remains pure FSK.
        if matches!(
            self.current_mode,
            ModemMode::V22 | ModemMode::V22bis | ModemMode::Bell212A
        ) {
            let qam_mode = match self.current_mode {
                ModemMode::V22 => QAMMode::V22,
                ModemMode::V22bis => QAMMode::V22bis,
                ModemMode::Bell212A => QAMMode::Bell212A,
                ModemMode::Bell103 => QAMMode::V22, // unreachable due to matches! guard
            };
            let carrier = qam_mode.carrier_freq_originate();
            self.qam_modulator = Some(QAMModulator::new(qam_mode, carrier, sample_rate));
            self.qam_demodulator = Some(QAMDemodulator::new(qam_mode, carrier, sample_rate));
        }
    }

    /// Public constructor used by tests and TAPI layer.
    pub fn new() -> Result<Self, String> {
        Ok(Self::new_internal())
    }

    pub fn get_state(&self) -> ModemState {
        *self.state.lock().unwrap()
    }

    pub fn init_audio(&mut self) -> Result<(), String> {
        let config = ModemAudioConfig::default();
        let mut engine = WasapiAudioEngine::new(config)
            .map_err(|e| format!("Audio engine creation failed: {}", e))?;

        engine
            .initialize()
            .map_err(|e| format!("Audio initialization failed: {}", e))?;
        engine
            .start()
            .map_err(|e| format!("Audio start failed: {}", e))?;

        self.audio_engine = Some(engine);
        Ok(())
    }

    pub fn process_command(&mut self, command: ATCommand) -> Vec<ATResponse> {
        let mut responses = Vec::new();
        log::info!("Processing command: {:?}", command);

        match command {
            ATCommand::Attention => {
                // Hayes "AT" prefix – just acknowledge.
                responses.push(ATResponse::Ok);
            }
            ATCommand::Dial(number) => {
                log::info!("Dialing: {}", number);
                *self.state.lock().unwrap() = ModemState::Connecting;
                self.connected = true;
                // For now, assume fast connect at current speed.
                *self.state.lock().unwrap() = ModemState::Connected;
                responses.push(ATResponse::Connect(self.connection_speed));
            }
            ATCommand::Answer => {
                log::info!("Answering call");
                *self.state.lock().unwrap() = ModemState::Connected;
                self.connected = true;
                responses.push(ATResponse::Connect(self.connection_speed));
            }
            ATCommand::Hangup => {
                self.hangup();
                responses.push(ATResponse::Ok);
            }
            ATCommand::SetEcho(on) => {
                log::debug!("Set echo: {}", on);
                responses.push(ATResponse::Ok);
            }
            ATCommand::SetVerbose(on) => {
                log::debug!("Set verbose: {}", on);
                responses.push(ATResponse::Ok);
            }
            ATCommand::SetSpeaker(on) => {
                log::debug!("Set speaker: {}", on);
                responses.push(ATResponse::Ok);
            }
            ATCommand::SelectSpeed(speed) => {
                log::info!("Select speed: {}", speed);
                self.reconfigure_for_speed(speed);
                responses.push(ATResponse::Ok);
            }
            ATCommand::Info(arg) => {
                // Simple "ATI<n>"-style info responses.
                let n: u32 = arg.trim().parse().unwrap_or(0);
                let info: String = match n {
                    0 => String::from("300"),
                    1 => String::from("OK"),
                    2 => String::from("OK"),
                    3 => String::from("HSF Softmodem v1.0"),
                    4 => String::from("Rust Implementation"),
                    _ => format!("Unknown info type {}", n),
                };
                responses.push(ATResponse::Text(info));
            }
            ATCommand::GoOnline => {
                *self.state.lock().unwrap() = ModemState::Connected;
                responses.push(ATResponse::Ok);
            }
            ATCommand::Reset => {
                self.reset();
                responses.push(ATResponse::Ok);
            }
            ATCommand::SetRegister(r, v) => {
                if (r as usize) < self.s_registers.len() {
                    self.s_registers[r as usize] = v;
                    responses.push(ATResponse::Ok);
                } else {
                    responses.push(ATResponse::Error);
                }
            }
            ATCommand::QueryRegister(r) => {
                if (r as usize) < self.s_registers.len() {
                    let value = self.s_registers[r as usize];
                    // 3-digit, zero-padded like many Hayes S-register displays.
                    responses.push(ATResponse::Text(format!("{:03}", value)));
                } else {
                    responses.push(ATResponse::Error);
                }
            }
            ATCommand::Unknown(_) => {
                responses.push(ATResponse::Error);
            }
        }

        responses
    }

    /// Process a byte while in data mode (Connected).
    /// Returns Some(responses) if an escape sequence was recognized.
    pub fn process_data_char(&mut self, byte: u8) -> Option<Vec<ATResponse>> {
        if byte == b'+' {
            self.plus_count += 1;
            if self.plus_count == 1 {
                self.escape_sequence_time = Some(Instant::now());
            } else if self.plus_count as usize == ESCAPE_SEQUENCE.len() {
                if let Some(start) = self.escape_sequence_time {
                    if start.elapsed() >= ESCAPE_GUARD_TIME {
                        log::info!("Escape sequence (+++) detected, returning to command mode");
                        self.hangup();
                        return Some(vec![ATResponse::Ok]);
                    }
                }
                // Either way, reset tracking once we've seen three '+' chars.
                self.plus_count = 0;
                self.escape_sequence_time = None;
            }
        } else {
            // Not '+': treat as data and reset escape tracking.
            self.plus_count = 0;
            self.escape_sequence_time = None;
            self.tx_buffer.push(byte);
        }

        None
    }

    /// Move pending TX bytes from host into an audio sample buffer for playback.
    pub fn process_tx_queue(&mut self) -> Vec<f32> {
        if self.tx_buffer.is_empty() {
            return Vec::new();
        }

        let data = std::mem::take(&mut self.tx_buffer);

        match self.current_mode {
            ModemMode::V22 | ModemMode::V22bis | ModemMode::Bell212A => {
                if let Some(ref mut modulator) = self.qam_modulator {
                    modulator.modulate_bytes(&data)
                } else {
                    Vec::new()
                }
            }
            _ => self.modulator.modulate_bytes(&data),
        }
    }

    /// Pull any captured audio from the engine and return newly demodulated bytes.
    pub fn process_audio(&mut self) -> Vec<u8> {
        if let Some(ref engine) = self.audio_engine {
            let events = engine.poll_events();
            for event in events {
                if let AudioEvent::CapturedSamples(samples) = event {
                    let bytes = match self.current_mode {
                        ModemMode::V22 | ModemMode::V22bis | ModemMode::Bell212A => {
                            if let Some(ref mut demod) = self.qam_demodulator {
                                demod.demodulate_bytes(&samples)
                            } else {
                                Vec::new()
                            }
                        }
                        _ => self.demodulator.demodulate_bytes(&samples),
                    };
                    self.rx_buffer.extend(bytes);
                }
            }
        }

        let data = self.rx_buffer.clone();
        self.rx_buffer.clear();
        data
    }

    /// Queue a block of audio samples for playback.
    pub fn queue_playback(&self, samples: Vec<f32>) {
        if let Some(ref engine) = self.audio_engine {
            engine.queue_playback(samples);
        }
    }

    pub fn hangup(&mut self) {
        self.connected = false;
        *self.state.lock().unwrap() = ModemState::Command;
    }

    pub fn reset(&mut self) {
        self.hangup();
        self.s_registers = Self::default_s_registers();
        self.parser = ATCommandParser::new();
        self.tx_buffer.clear();
        self.rx_buffer.clear();
        self.escape_sequence_time = None;
        self.plus_count = 0;
    }
}
