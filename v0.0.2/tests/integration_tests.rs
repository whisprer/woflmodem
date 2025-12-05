// tests/integration_tests.rs
//
// Lightweight integration tests for the VirtualModem public API.

use hsf_softmodem::tapi::at_commands::{ATCommand, ATResponse, ModemState};
use hsf_softmodem::tapi::modem::VirtualModem;

#[test]
fn test_modem_initialization() {
    let modem = VirtualModem::new().expect("modem should init");
    assert_eq!(modem.get_state(), ModemState::Command);
}

#[test]
fn test_speed_selection_via_command() {
    let mut modem = VirtualModem::new().unwrap();

    let responses = modem.process_command(ATCommand::SelectSpeed(1200));
    assert_eq!(responses, vec![ATResponse::Ok]);

    let responses = modem.process_command(ATCommand::SelectSpeed(2400));
    assert_eq!(responses, vec![ATResponse::Ok]);

    let responses = modem.process_command(ATCommand::SelectSpeed(300));
    assert_eq!(responses, vec![ATResponse::Ok]);
}

#[test]
fn test_dial_and_connect_flow() {
    let mut modem = VirtualModem::new().unwrap();

    assert_eq!(modem.get_state(), ModemState::Command);

    let responses = modem.process_command(ATCommand::Dial("5551234".to_string()));
    assert!(responses.iter().any(|r| matches!(r, ATResponse::Connect(_))));

    assert_eq!(modem.get_state(), ModemState::Connected);
}

#[test]
fn test_info_command_via_modem() {
    let mut modem = VirtualModem::new().unwrap();

    let responses = modem.process_command(ATCommand::Info("3".to_string()));
    assert_eq!(responses.len(), 1);

    match &responses[0] {
        ATResponse::Text(text) => assert!(text.contains("HSF Softmodem")),
        other => panic!("Expected Text response, got {:?}", other),
    }
}

#[test]
fn test_s_register_access_round_trip() {
    let mut modem = VirtualModem::new().unwrap();

    let responses = modem.process_command(ATCommand::SetRegister(0, 3));
    assert_eq!(responses, vec![ATResponse::Ok]);

    let responses = modem.process_command(ATCommand::QueryRegister(0));
    assert_eq!(responses.len(), 1);

    match &responses[0] {
        ATResponse::Text(val) => assert_eq!(val, "003"),
        other => panic!("Expected Text response, got {:?}", other),
    }
}
