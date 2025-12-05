// tests/at_command_test.rs
//
// Updated to match the current Hayes-style AT parser and response types.

use hsf_softmodem::tapi::at_commands::{ATCommand, ATCommandParser, ATResponse};

#[test]
fn test_basic_at_commands() {
    let mut parser = ATCommandParser::new();

    assert_eq!(parser.parse_command_line("AT"), vec![ATCommand::Attention]);
    assert_eq!(parser.parse_command_line("ATZ"), vec![ATCommand::Reset]);
    assert_eq!(parser.parse_command_line("ATA"), vec![ATCommand::Answer]);
    assert_eq!(parser.parse_command_line("ATH"), vec![ATCommand::Hangup]);
    assert_eq!(parser.parse_command_line("ATO"), vec![ATCommand::GoOnline]);
}

#[test]
fn test_dial_command_tone_and_pulse_prefixes_are_ignored() {
    let mut parser = ATCommandParser::new();

    let cmds = parser.parse_command_line("ATDT5551234");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        ATCommand::Dial(number) => assert_eq!(number, "5551234"),
        other => panic!("Expected Dial command, got {:?}", other),
    }

    let cmds = parser.parse_command_line("ATDP9876543");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        ATCommand::Dial(number) => assert_eq!(number, "9876543"),
        other => panic!("Expected Dial command, got {:?}", other),
    }
}

#[test]
fn test_s_register_commands() {
    let mut parser = ATCommandParser::new();

    let cmds = parser.parse_command_line("ATS0=2");
    assert_eq!(cmds.len(), 1);
    match cmds[0] {
        ATCommand::SetRegister(reg, val) => {
            assert_eq!(reg, 0);
            assert_eq!(val, 2);
        }
        _ => panic!("Expected SetRegister"),
    }

    let cmds = parser.parse_command_line("ATS7?");
    assert_eq!(cmds, vec![ATCommand::QueryRegister(7)]);
}

#[test]
fn test_echo_verbose_speaker_commands() {
    let mut parser = ATCommandParser::new();

    assert_eq!(parser.parse_command_line("ATE0"), vec![ATCommand::SetEcho(false)]);
    assert_eq!(parser.parse_command_line("ATE1"), vec![ATCommand::SetEcho(true)]);

    assert_eq!(parser.parse_command_line("ATV0"), vec![ATCommand::SetVerbose(false)]);
    assert_eq!(parser.parse_command_line("ATV1"), vec![ATCommand::SetVerbose(true)]);

    assert_eq!(parser.parse_command_line("ATM0"), vec![ATCommand::SetSpeaker(false)]);
    assert_eq!(parser.parse_command_line("ATM1"), vec![ATCommand::SetSpeaker(true)]);
}

#[test]
fn test_compound_commands() {
    let mut parser = ATCommandParser::new();

    let cmds = parser.parse_command_line("ATE1V1Z");
    assert!(cmds.len() >= 3);

    assert!(cmds.iter().any(|c| matches!(c, ATCommand::SetEcho(true))));
    assert!(cmds.iter().any(|c| matches!(c, ATCommand::SetVerbose(true))));
    assert!(cmds.iter().any(|c| matches!(c, ATCommand::Reset)));
}

#[test]
fn test_extended_speed_selection() {
    let mut parser = ATCommandParser::new();

    let cmds = parser.parse_command_line("AT+MS=300");
    assert_eq!(cmds, vec![ATCommand::SelectSpeed(300)]);
}

#[test]
fn test_streaming_parser_process_char() {
    let mut parser = ATCommandParser::new();

    let mut out: Option<Vec<ATCommand>> = None;
    for ch in "ATZ\r".chars() {
        if let Some(cmds) = parser.process_char(ch) {
            out = Some(cmds);
        }
    }

    assert_eq!(out.unwrap(), vec![ATCommand::Reset]);
}

#[test]
fn test_response_formatting() {
    let resp = ATResponse::Ok;
    assert_eq!(resp.to_string(), "OK\r\n");

    let resp = ATResponse::Connect(2400);
    assert_eq!(resp.to_string(), "CONNECT 2400\r\n");

    let resp = ATResponse::Error;
    assert_eq!(resp.to_string(), "ERROR\r\n");

    let resp = ATResponse::Text("HSF Softmodem".to_string());
    assert_eq!(resp.to_string(), "HSF Softmodem\r\n");
}
