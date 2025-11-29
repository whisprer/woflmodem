// src/tapi/at_commands.rs
//
// Hayes-style AT command parsing and response types.

use std::fmt;

/// Hayes AT command responses.
#[derive(Debug, Clone, PartialEq)]
pub enum ATResponse {
    Ok,
    Error,
    Connect(u32), // Connect with baud rate
    Ring,
    NoCarrier,
    NoDialtone,
    Busy,
    NoAnswer,
    Text(String),
}

impl ATResponse {
    /// Render the response as a full modem-style line.
    pub fn to_string(&self) -> String {
        match self {
            ATResponse::Ok => "OK\r\n".to_string(),
            ATResponse::Error => "ERROR\r\n".to_string(),
            ATResponse::Connect(baud) => format!("CONNECT {}\r\n", baud),
            ATResponse::Ring => "RING\r\n".to_string(),
            ATResponse::NoCarrier => "NO CARRIER\r\n".to_string(),
            ATResponse::NoDialtone => "NO DIALTONE\r\n".to_string(),
            ATResponse::Busy => "BUSY\r\n".to_string(),
            ATResponse::NoAnswer => "NO ANSWER\r\n".to_string(),
            ATResponse::Text(s) => format!("{}\r\n", s),
        }
    }
}

/// Logical modem state as seen by the TAPI / control layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModemState {
    Command,
    Dialing,
    Ringing,
    Connecting,
    Connected,
    OnHook,
    OffHook,
}

/// Parsed Hayes AT command.
#[derive(Debug, Clone, PartialEq)]
pub enum ATCommand {
    Attention,
    Dial(String),
    Answer,
    Hangup,
    SetEcho(bool),
    SetVerbose(bool),
    SetSpeaker(bool),
    SelectSpeed(u32),    // From parsing +MS=<speed>
    Info(String),        // ATI<n>
    GoOnline,            // "O"
    Reset,               // "Z"
    SetRegister(u8, u8), // S<n>=<v>
    QueryRegister(u8),   // S<n>?
    Unknown(String),
}

impl fmt::Display for ATCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ATCommand::Attention => write!(f, "AT"),
            ATCommand::Dial(num) => write!(f, "ATD{}", num),
            ATCommand::Answer => write!(f, "ATA"),
            ATCommand::Hangup => write!(f, "ATH"),
            ATCommand::SetEcho(v) => write!(f, "ATE{}", if *v { 1 } else { 0 }),
            ATCommand::SetVerbose(v) => write!(f, "ATV{}", if *v { 1 } else { 0 }),
            ATCommand::SetSpeaker(v) => write!(f, "ATM{}", if *v { 1 } else { 0 }),
            ATCommand::SelectSpeed(s) => write!(f, "AT+MS={}", s),
            ATCommand::Info(i) => write!(f, "ATI{}", i),
            ATCommand::GoOnline => write!(f, "ATO"),
            ATCommand::Reset => write!(f, "ATZ"),
            ATCommand::SetRegister(r, v) => write!(f, "ATS{}={}", r, v),
            ATCommand::QueryRegister(r) => write!(f, "ATS{}?", r),
            ATCommand::Unknown(s) => write!(f, "AT{}", s),
        }
    }
}

/// Streaming AT command parser, fed one character at a time.
#[derive(Debug, Default)]
pub struct ATCommandParser {
    command_buffer: String,
}

impl ATCommandParser {
    pub fn new() -> Self {
        Self {
            command_buffer: String::new(),
        }
    }

    /// Process a single character and return parsed commands when a line is complete.
    pub fn process_char(&mut self, ch: char) -> Option<Vec<ATCommand>> {
        match ch {
            '\r' | '\n' => {
                let line: String = self.command_buffer.drain(..).collect();
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                Some(self.parse_line(line))
            }
            _ => {
                self.command_buffer.push(ch);
                None
            }
        }
    }

    /// Parse a full command line, used by tests like:
    ///   parser.parse_command_line("ATZ");
    pub fn parse_command_line(&mut self, line: &str) -> Vec<ATCommand> {
        self.parse_line(line.trim())
    }

    fn parse_line(&self, line: &str) -> Vec<ATCommand> {
        let mut cmds = Vec::new();
        let s = line.trim();

        if s.is_empty() {
            return cmds;
        }

        // All commands must start with AT (case-insensitive).
        if !s.to_ascii_uppercase().starts_with("AT") {
            cmds.push(ATCommand::Unknown(s.to_string()));
            return cmds;
        }

        // Strip single leading "AT".
        let body = &s[2..];

        if body.is_empty() {
            cmds.push(ATCommand::Attention);
            return cmds;
        }

        // Many Hayes sequences compress multiple commands: ATZV1E1 etc.
        // We'll walk the body left to right and emit a command per prefix.
        let mut i = 0;
        let chars: Vec<char> = body.chars().collect();
        let len = chars.len();

        while i < len {
            let c = chars[i].to_ascii_uppercase();
            i += 1;

            match c {
                'Z' => {
                    cmds.push(ATCommand::Reset);
                }
                'A' => {
                    cmds.push(ATCommand::Answer);
                }
                'H' => {
                    cmds.push(ATCommand::Hangup);
                }
                'O' => {
                    cmds.push(ATCommand::GoOnline);
                }
                'E' => {
                    // ATE0 / ATE1
                    let mut echo_on = true;
                    if i < len {
                        if chars[i] == '0' {
                            echo_on = false;
                            i += 1;
                        } else if chars[i] == '1' {
                            echo_on = true;
                            i += 1;
                        }
                    }
                    cmds.push(ATCommand::SetEcho(echo_on));
                }
                'V' => {
                    // ATV0 / ATV1
                    let mut verbose_on = true;
                    if i < len {
                        if chars[i] == '0' {
                            verbose_on = false;
                            i += 1;
                        } else if chars[i] == '1' {
                            verbose_on = true;
                            i += 1;
                        }
                    }
                    cmds.push(ATCommand::SetVerbose(verbose_on));
                }
                'M' => {
                    // ATM0 / ATM1
                    let mut speaker_on = true;
                    if i < len {
                        if chars[i] == '0' {
                            speaker_on = false;
                            i += 1;
                        } else if chars[i] == '1' {
                            speaker_on = true;
                            i += 1;
                        }
                    }
                    cmds.push(ATCommand::SetSpeaker(speaker_on));
                }
                'D' => {
                    // Dial: everything after D (and optional T/P) up to end.
                    let mut digits = String::new();

                    // Optional T/P
                    if i < len
                        && (chars[i].eq_ignore_ascii_case(&'T')
                            || chars[i].eq_ignore_ascii_case(&'P'))
                    {
                        i += 1;
                    }

                    while i < len {
                        digits.push(chars[i]);
                        i += 1;
                    }

                    cmds.push(ATCommand::Dial(digits));
                    break; // Dial usually consumes the rest of the line.
                }
                'I' => {
                    // ATI<n> – info query
                    let mut digits = String::new();
                    while i < len && chars[i].is_ascii_digit() {
                        digits.push(chars[i]);
                        i += 1;
                    }
                    if digits.is_empty() {
                        digits.push('0');
                    }
                    cmds.push(ATCommand::Info(digits));
                }
                'S' => {
                    // S<n>=<v> or S<n>?
                    let mut reg_digits = String::new();
                    while i < len && chars[i].is_ascii_digit() {
                        reg_digits.push(chars[i]);
                        i += 1;
                    }
                    if let Ok(reg) = reg_digits.parse::<u8>() {
                        if i < len && chars[i] == '=' {
                            i += 1;
                            let mut val_digits = String::new();
                            while i < len && chars[i].is_ascii_digit() {
                                val_digits.push(chars[i]);
                                i += 1;
                            }
                            if let Ok(val) = val_digits.parse::<u8>() {
                                cmds.push(ATCommand::SetRegister(reg, val));
                            } else {
                                cmds.push(ATCommand::Unknown(format!("S{}={}", reg, val_digits)));
                            }
                        } else if i < len && chars[i] == '?' {
                            i += 1;
                            cmds.push(ATCommand::QueryRegister(reg));
                        } else {
                            cmds.push(ATCommand::Unknown(format!("S{}", reg)));
                        }
                    } else {
                        cmds.push(ATCommand::Unknown(format!("S{}", reg_digits)));
                    }
                }
                '+' => {
                    // Extended commands, e.g. +MS=300
                    let start = i - 1;
                    let mut end = start;
                    while end < len {
                        let ch = chars[end];
                        if ch == ';' {
                            break;
                        }
                        end += 1;
                    }
                    let token: String = chars[start..end].iter().collect();
                    i = end;

                    if let Some(cmd) = Self::parse_extended(&token) {
                        cmds.push(cmd);
                    } else {
                        cmds.push(ATCommand::Unknown(token));
                    }
                }
                ';' => {
                    // Command separator – ignore.
                }
                _ => {
                    // Unrecognized – treat the rest as unknown.
                    let tail: String = std::iter::once(c)
                        .chain(chars[i..].iter().cloned())
                        .collect();
                    cmds.push(ATCommand::Unknown(tail));
                    break;
                }
            }
        }

        if cmds.is_empty() {
            cmds.push(ATCommand::Unknown(body.to_string()));
        }

        cmds
    }

    fn parse_extended(s: &str) -> Option<ATCommand> {
        let upper = s.to_ascii_uppercase();

        // +MS=<speed>
        if let Some(rest) = upper.strip_prefix("+MS=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(speed) = digits.parse::<u32>() {
                return Some(ATCommand::SelectSpeed(speed));
            }
        }

        None
    }

    pub fn reset(&mut self) {
        self.command_buffer.clear();
    }
}
