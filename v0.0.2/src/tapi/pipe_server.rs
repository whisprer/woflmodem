// src/tapi/pipe_server.rs
//
// Windows named-pipe "virtual COM" front-end for the VirtualModem.
// This version preserves the v0.0.1-style public API so main.rs
// does not need to change:
//
// - ModemPipeServer::new() -> io::Result<Self>
// - server.run() -> io::Result<()>
//
// It also uses the windows 0.58 safe wrapper signatures correctly,
// avoids overlapped I/O, and bridges text -> ATCommand -> ATResponse.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;

use log::{debug, info};

use windows::core::{PCWSTR, Result as WinResult};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE,
    PIPE_WAIT,
};

use crate::tapi::at_commands::ATCommandParser;
use crate::tapi::modem::VirtualModem;

pub struct ModemPipeServer {
    modem: VirtualModem,
    pipe_name: String,
    out_buf_size: u32,
    in_buf_size: u32,
    max_instances: u32,
}

impl ModemPipeServer {
    /// Backward-compatible constructor expected by your main.rs.
    pub fn new() -> io::Result<Self> {
        // Assume the v0.0.x VirtualModem constructor shape:
        // Result<Self, String>
        let mut modem = VirtualModem::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // If your v0.0.2 modem still has init_audio(), this keeps behavior aligned
        // with earlier revisions. If it was removed, comment out this block.
        if let Err(e) = modem.init_audio() {
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }

        Ok(Self {
            modem,
            pipe_name: "HsfSoftmodem".to_string(),
            out_buf_size: 4096,
            in_buf_size: 4096,
            max_instances: 1,
        })
    }

    fn full_pipe_name(&self) -> String {
        format!(r"\\.\pipe\{}", self.pipe_name)
    }

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn create_pipe(&self) -> WinResult<HANDLE> {
        let full = self.full_pipe_name();
        let wide = Self::to_wide(&full);

        // WinAPI value: PIPE_ACCESS_DUPLEX = 0x0000_0003.
        const PIPE_ACCESS_DUPLEX_U32: u32 = 0x0000_0003;

        let open_mode = FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX_U32);
        let pipe_mode = PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT;

        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(wide.as_ptr()),
                open_mode,
                pipe_mode,
                self.max_instances,
                self.out_buf_size,
                self.in_buf_size,
                0,
                None,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(windows::core::Error::from_win32());
        }

        Ok(handle)
    }

    fn connect_pipe(handle: HANDLE) -> WinResult<()> {
        match unsafe { ConnectNamedPipe(handle, None) } {
            Ok(()) => Ok(()),
            Err(e) => {
                let err = unsafe { GetLastError() };
                if err == ERROR_PIPE_CONNECTED {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn read_client(handle: HANDLE, buf: &mut [u8]) -> WinResult<u32> {
        let mut read: u32 = 0;

        match unsafe { ReadFile(handle, Some(buf), Some(&mut read), None) } {
            Ok(()) => Ok(read),
            Err(e) => {
                let err = unsafe { GetLastError() };
                if err == ERROR_BROKEN_PIPE {
                    Ok(0)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn write_client(handle: HANDLE, data: &[u8]) -> WinResult<()> {
        let mut written: u32 = 0;
        unsafe { WriteFile(handle, Some(data), Some(&mut written), None) }?;

        if written as usize != data.len() {
            return Err(windows::core::Error::from_win32());
        }

        Ok(())
    }

    /// Backward-compatible run loop expected by your main.rs.
    pub fn run(&mut self) -> io::Result<()> {
        let full = self.full_pipe_name();
        info!("Named pipe server listening on {}", full);

        loop {
            let handle = self.create_pipe()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?;

            debug!("Waiting for pipe client...");
            if let Err(e) = Self::connect_pipe(handle) {
                unsafe { CloseHandle(handle); }
                return Err(io::Error::new(io::ErrorKind::Other, format!("{e:?}")));
            }

            debug!("Pipe client connected.");

            let mut rx = vec![0u8; self.in_buf_size as usize];
            let mut line_buf = Vec::<u8>::with_capacity(512);

            // Per-client read loop
            loop {
                let n = Self::read_client(handle, &mut rx)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))? as usize;

                if n == 0 {
                    debug!("Client disconnected.");
                    break;
                }

                line_buf.extend_from_slice(&rx[..n]);

                // AT lines are usually CR or CRLF terminated.
                while let Some(pos) = line_buf.iter().position(|&b| b == b'\r' || b == b'\n') {
                    let mut raw = line_buf.drain(..=pos).collect::<Vec<u8>>();

                    while matches!(raw.last(), Some(b'\r' | b'\n')) {
                        raw.pop();
                    }

                    if raw.is_empty() {
                        continue;
                    }

                    let cmd_line = String::from_utf8_lossy(&raw).to_string();
                    debug!("RX AT line: {:?}", cmd_line);

                    // Parse -> execute -> collect responses
                    let mut parser = ATCommandParser::new();
                    let commands = parser.parse_command_line(&cmd_line);

                    let mut responses = Vec::new();
                    for c in commands {
                        responses.extend(self.modem.process_command(c));
                    }

                    // Serialize responses to wire text
                    let mut out = String::new();
                    for r in &responses {
                        out.push_str(&r.to_string());
                        out.push_str("\r\n");
                    }

                    // Ensure at least one line terminator
                    if out.is_empty() {
                        out.push_str("OK\r\n");
                    }

                    Self::write_client(handle, out.as_bytes())
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?;
                }

                // Guard against runaway growth if a client sends no line breaks.
                if line_buf.len() > 64 * 1024 {
                    line_buf.clear();
                }
            }

            unsafe {
                DisconnectNamedPipe(handle).ok();
                CloseHandle(handle);
            }
        }
    }
}
