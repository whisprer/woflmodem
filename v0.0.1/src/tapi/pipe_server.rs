// src/tapi/pipe_server.rs
//
// Windows named-pipe "virtual COM" front-end for the VirtualModem.

use super::modem::VirtualModem;
use super::at_commands::{ModemState};
use std::io;
use std::thread;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

pub struct ModemPipeServer {
    modem: VirtualModem,
}

impl ModemPipeServer {
    pub fn new() -> io::Result<Self> {
        let mut modem = VirtualModem::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        modem
            .init_audio()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(Self { modem })
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Build a wide string for the pipe name.
        let name_utf16: Vec<u16> = r"\\.\pipe\HsfSoftmodem"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let pipe_name = PCWSTR(name_utf16.as_ptr());

        loop {
            let pipe = unsafe {
                CreateNamedPipeW(
                    pipe_name,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    4096,
                    4096,
                    0,
                    None,
                )
            };

            if pipe == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }

            log::info!("Waiting for TAPI / virtual COM client to connect...");

            unsafe { ConnectNamedPipe(pipe, None) }
                .map_err(|_| io::Error::last_os_error())?;

            log::info!("Client connected.");

            let result = self.handle_client(pipe);

            unsafe {
                let _ = DisconnectNamedPipe(pipe);
                let _ = CloseHandle(pipe);
            }

            if let Err(e) = result {
                log::error!("Client session ended with error: {}", e);
            }

            // Be gentle between client sessions.
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn handle_client(&mut self, pipe: HANDLE) -> io::Result<()> {
        let mut buffer = [0u8; 1024];

        loop {
            let mut bytes_read: u32 = 0;
            let read_result =
                unsafe { ReadFile(pipe, Some(&mut buffer), Some(&mut bytes_read), None) };

            match read_result {
                Ok(()) => {
                    if bytes_read == 0 {
                        // Client closed the pipe.
                        return Ok(());
                    }
                }
                Err(_) => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::BrokenPipe {
                        return Ok(());
                    } else {
                        return Err(err);
                    }
                }
            }

            let data = &buffer[..bytes_read as usize];
            for &b in data {
                self.process_incoming_byte(pipe, b)?;
            }

            // After consuming incoming bytes, give the modem a chance to
            // move data between host and audio paths.
            self.pump_modem_audio(pipe)?;
        }
    }

    fn process_incoming_byte(&mut self, pipe: HANDLE, byte: u8) -> io::Result<()> {
        let state = self.modem.get_state();

        match state {
            ModemState::Command => {
                let ch = byte as char;

                if let Some(commands) = self.modem.parser.process_char(ch) {
                    for cmd in commands {
                        let responses = self.modem.process_command(cmd);
                        for resp in responses {
                            let line = resp.to_string();
                            self.write_bytes(pipe, line.as_bytes())?;
                        }
                    }
                }
            }
            ModemState::Connected => {
                if let Some(responses) = self.modem.process_data_char(byte) {
                    for resp in responses {
                        let line = resp.to_string();
                        self.write_bytes(pipe, line.as_bytes())?;
                    }
                }
            }
            _ => {
                // Other states (Dialing, Ringing, etc.) are handled internally for now.
            }
        }

        Ok(())
    }

    fn pump_modem_audio(&mut self, pipe: HANDLE) -> io::Result<()> {
        // Transmit side: host -> modem -> audio
        let tx_samples = self.modem.process_tx_queue();
        if !tx_samples.is_empty() {
            self.modem.queue_playback(tx_samples);
        }

        // Receive side: audio -> modem -> host
        let rx_bytes = self.modem.process_audio();
        if !rx_bytes.is_empty() {
            self.write_bytes(pipe, &rx_bytes)?;
        }

        Ok(())
    }

    fn write_bytes(&self, pipe: HANDLE, data: &[u8]) -> io::Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        unsafe {
            let mut bytes_written: u32 = 0;
            WriteFile(pipe, Some(data), Some(&mut bytes_written), None)
                .map_err(|_| io::Error::last_os_error())?;
        }

        Ok(())
    }
}
