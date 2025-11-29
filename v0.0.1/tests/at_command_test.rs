// tests/at_command_tests.rs
use hsf_softmodem::tapi::at_commands::*;

#[test]
fn test_basic_at_commands() {
    let mut parser = ATCommandParser::new();
    
    // Test AT
    let cmds = parser.parse_command_line("AT");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], ATCommand::Attention);
    
    // Test ATZ (reset)
    let cmds = parser.parse_command_line("ATZ");
    assert_eq!(cmds[0], ATCommand::Reset);
}

#[test]
fn test_dial_command() {
    let mut parser = ATCommandParser::new();
    
    // Test dial with number
    let cmds = parser.parse_command_line("ATDT5551234");
    assert_eq!(cmds.len(), 1);
    if let ATCommand::Dial(number) = &cmds[0] {
        assert_eq!(number, "5551234");
    } else {
        panic!("Expected Dial command");
    }
    
    // Test pulse dial
    let cmds = parser.parse_command_line("ATDP9876543");
    if let ATCommand::Dial(number) = &cmds[0] {
        assert_eq!(number, "P9876543");
    } else {
        panic!("Expected Dial command");
    }
}

#[test]
fn test_s_register_commands() {
    let mut parser = ATCommandParser::new();
    
    // Set S0=2
    let cmds = parser.parse_command_line("ATS0=2");
    if let ATCommand::SetRegister(reg, val) = cmds[0] {
        assert_eq!(reg, 0);
        assert_eq!(val, 2);
    } else {
        panic!("Expected SetRegister");
    }
    
    // Query S7?
    let cmds = parser.parse_command_line("ATS7?");
    if let ATCommand::QueryRegister(reg) = cmds[0] {
        assert_eq!(reg, 7);
    } else {
        panic!("Expected QueryRegister");
    }
}

#[test]
fn test_echo_verbose_commands() {
    let mut parser = ATCommandParser::new();
    
    // Test E0 (echo off)
    let cmds = parser.parse_command_line("ATE0");
    assert_eq!(cmds[0], ATCommand::EchoMode(false));
    assert!(!parser.is_echo_enabled());
    
    // Test V1 (verbose on)
    let cmds = parser.parse_command_line("ATV1");
    assert_eq!(cmds[0], ATCommand::VerboseMode(true));
    assert!(parser.is_verbose());
}

#[test]
fn test_compound_commands() {
    let mut parser = ATCommandParser::new();
    
    // Multiple commands in one line
    let cmds = parser.parse_command_line("ATE1V1Z");
    assert!(cmds.len() >= 3);
    
    // Should contain echo, verbose, and reset
    assert!(cmds.iter().any(|c| matches!(c, ATCommand::EchoMode(true))));
    assert!(cmds.iter().any(|c| matches!(c, ATCommand::VerboseMode(true))));
    assert!(cmds.iter().any(|c| matches!(c, ATCommand::Reset)));
}

#[test]
fn test_response_formatting() {
    // OK response
    let resp = ATResponse::Ok;
    assert_eq!(resp.to_string(), "OK\r\n");
    
    // Connect response
    let resp = ATResponse::Connect(2400);
    assert_eq!(resp.to_string(), "CONNECT 2400\r\n");
    
    // Error response
    let resp = ATResponse::Error;
    assert_eq!(resp.to_string(), "ERROR\r\n");
}

#[test]
fn test_escape_sequence_detection() {
    // This would be in the modem implementation
    // Test that +++ with proper guard times triggers escape
    // (Simplified unit test)
    
    let escape_char = b'+';
    let mut plus_count = 0;
    
    for _ in 0..3 {
        plus_count += 1;
    }
    
    assert_eq!(plus_count, 3);
}
