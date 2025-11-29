// tests/integration_tests.rs
use hsf_softmodem::tapi::modem::*;
use hsf_softmodem::tapi::at_commands::*;

#[test]
fn test_modem_initialization() {
    let result = VirtualModem::new();
    assert!(result.is_ok());
    
    let modem = result.unwrap();
    assert_eq!(modem.get_state(), ModemState::Command);
}

#[test]
fn test_modem_mode_switching() {
    let mut modem = VirtualModem::new().unwrap();
    
    // Test V.22
    assert!(modem.set_mode(ModemMode::V22).is_ok());
    
    // Test V.22bis
    assert!(modem.set_mode(ModemMode::V22bis).is_ok());
    
    // Test Bell 103
    assert!(modem.set_mode(ModemMode::Bell103).is_ok());
}

#[test]
fn test_modem_command_processing() {
    let mut modem = VirtualModem::new().unwrap();
    
    // Test ATZ (reset)
    let responses = modem.process_command(ATCommand::Reset);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], ATResponse::Ok);
    
    // Test ATI (info)
    let responses = modem.process_command(ATCommand::Info(3));
    assert_eq!(responses.len(), 1);
    if let ATResponse::Text(text) = &responses[0] {
        assert!(text.contains("HSF Softmodem"));
    }
}

#[test]
fn test_s_register_access() {
    let mut modem = VirtualModem::new().unwrap();
    
    // Set S0=3
    let responses = modem.process_command(ATCommand::SetRegister(0, 3));
    assert_eq!(responses[0], ATResponse::Ok);
    
    // Query S0?
    let responses = modem.process_command(ATCommand::QueryRegister(0));
    if let ATResponse::Text(val) = &responses[0] {
        assert_eq!(val, "003");
    }
}
