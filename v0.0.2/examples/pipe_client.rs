//! Simple Named Pipe client for the HSF Softmodem AT control channel.
//!
//! Usage:
//!   cargo run --example pipe_client --release
//!   cargo run --example pipe_client --release -- AT ATI3 ATZ
//!
//! This expects the server to be listening on:
//!   \\.\pipe\HsfSoftmodem
//!
//! It uses the existing `windows` crate in your project.
//! No new dependencies.

use windows::core::{PCWSTR, Result as WinResult};

use windows::Win32::Foundation::{CloseHandle, HANDLE};

use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile,
    FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

use windows::Win32::System::Pipes::{
    PeekNamedPipe, SetNamedPipeHandleState, WaitNamedPipeW,
    PIPE_READMODE_MESSAGE, NAMED_PIPE_MODE,
};


fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn open_pipe(pipe_base_name: &str) -> WinResult<HANDLE> {
    let full = format!(r"\\.\pipe\{}", pipe_base_name);
    let wide = to_wide(&full);

    // Give the server a moment to publish the instance.
    for _ in 0..5 {
        let ok = unsafe { WaitNamedPipeW(PCWSTR(wide.as_ptr()), 2000) }.as_bool();
        if ok {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    // windows 0.58: CreateFileW returns Result<HANDLE>
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            (FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0) as u32,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    // Message read mode to match PIPE_TYPE_MESSAGE server
    unsafe {
        let mode: NAMED_PIPE_MODE = PIPE_READMODE_MESSAGE;
        SetNamedPipeHandleState(handle, Some(&mode), None, None)?;
    }

    Ok(handle)
}

fn write_at_line(handle: HANDLE, line: &str) -> windows::core::Result<()> {
    let mut bytes = line.as_bytes().to_vec();
    if !bytes.ends_with(b"\r") {
        bytes.push(b'\r');
    }

    let mut written: u32 = 0;
    unsafe { WriteFile(handle, Some(bytes.as_slice()), Some(&mut written), None)?; }

    Ok(())
}

/// Read whatever the server has already produced, without blocking.
/// We poll for up to `timeout_ms` total, returning accumulated bytes.
fn read_available(handle: HANDLE, timeout_ms: u64) -> WinResult<Vec<u8>> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    let mut out = Vec::<u8>::new();

    loop {
        let mut avail: u32 = 0;

        unsafe {
            // windows 0.58 signature:
            // PeekNamedPipe(h, lpbuffer, nbuffersize, lpbytesread, lptotalbytesavail, lpbytesleftthismessage)
            PeekNamedPipe(handle, None, 0, None, Some(&mut avail), None)?;
        }

        if avail > 0 {
            let mut buf = vec![0u8; avail as usize];
            let mut read: u32 = 0;
            unsafe { ReadFile(handle, Some(buf.as_mut_slice()), Some(&mut read), None)?; }
            buf.truncate(read as usize);
            out.extend_from_slice(&buf);

            thread::sleep(Duration::from_millis(10));
            continue;
        }

        if start.elapsed() >= timeout {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }

    Ok(out)
}

fn print_response_bytes(bytes: &[u8]) {
    if bytes.is_empty() {
        println!("<no response yet>");
        return;
    }

    // Responses are line-oriented; tolerate CR, LF, or CRLF.
    let s = String::from_utf8_lossy(bytes);
    for line in s.lines() {
        let t = line.trim_matches(&['\r', '\n'][..]).trim();
        if !t.is_empty() {
            println!("{}", t);
        }
    }
}

fn main() -> windows::core::Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    // Allow optional "--" separator.
    if args.first().map(|s| s.as_str()) == Some("--") {
        args.remove(0);
    }

    let commands = if args.is_empty() {
        vec!["AT".to_string(), "ATI3".to_string(), "ATZ".to_string()]
    } else {
        args
    };

    let pipe_name = "HsfSoftmodem";
    println!("Connecting to \\\\.\\pipe\\{} ...", pipe_name);

    let handle = open_pipe(pipe_name)?;
    println!("Connected.");

    for cmd in commands {
        println!("\n> {}", cmd);
        write_at_line(handle, &cmd)?;

        // Give the server a moment to process and respond.
        let bytes = read_available(handle, 200)?;
        print_response_bytes(&bytes);
    }

    unsafe { CloseHandle(handle); }
    Ok(())
}
