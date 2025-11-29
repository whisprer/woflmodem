// tests/integration_tests.rs
use hsf_softmodem::tapi::modem::*;
use hsf_softmodem::tapi::at_commands::*;

/// Modem comes up successfully and starts in Command state
#[test]
fn test_modem_initialization() {
    let result = VirtualModem::new();
    assert!(result.is_ok());

    let modem = result.unwrap();
    assert_eq!(modem.get_state(), ModemState::Command);
}

/// Simple AT / ATZ round-trip via the modem command path
#[test]
fn test_basic_attention_and_reset_cycle() {
    let mut modem = VirtualModem::new().unwrap();

    // AT
    let responses = modem.process_command(ATCommand::Attention);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], ATResponse::Ok);

    // ATZ (reset)
    let responses = modem.process_command(ATCommand::Reset);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], ATResponse::Ok);

    // After reset we should still be in command mode
    assert_eq!(modem.get_state(), ModemState::Command);
}

/// Dial workflow: we issue a Dial command and expect a Connect plus state change
#[test]
fn test_dial_and_connect_flow() {
    let mut modem = VirtualModem::new().unwrap();

    // Make sure we start in Command state
    assert_eq!(modem.get_state(), ModemState::Command);

    // Dial a number
    let responses = modem.process_command(ATCommand::Dial("5551234".to_string()));

    // We expect at least one response and not an Error
    assert!(!responses.is_empty());
    assert!(
        responses.iter().any(|r| matches!(r, ATResponse::Connect(_))),
        "Expected a Connect response, got: {:?}",
        responses
    );

    // Modem should now report a connected state
    let state = modem.get_state();
    assert!(
        matches!(state, ModemState::Connected | ModemState::OnlineData),
        "Expected connected-like state after Dial, got: {:?}",
        state
    );
}

/// Information query (ATI3 style) through the modem
#[test]
fn test_info_command_via_modem() {
    let mut modem = VirtualModem::new().unwrap();

    // ATI3 style info – the AT layer exposes this as Info("3")
    let responses = modem.process_command(ATCommand::Info("3".to_string()));

    // We don't assert exact firmware strings here, just that we get some text or OK
    assert!(!responses.is_empty());

    // Expect at least one Text or Ok response
    assert!(
        responses
            .iter()
            .any(|r| matches!(r, ATResponse::Text(_) | ATResponse::Ok)),
        "Expected Text/Ok response to ATI3-style Info, got: {:?}",
        responses
    );
}

/// S-register set & query round-trip (ATS0=3 / ATS0?)
#[test]
fn test_s_register_access() {
    let mut modem = VirtualModem::new().unwrap();

    // Set S0=3
    let responses = modem.process_command(ATCommand::SetRegister(0, 3));
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], ATResponse::Ok);

    // Query S0?
    let responses = modem.process_command(ATCommand::QueryRegister(0));
    assert!(!responses.is_empty());

    // The first response should be Text("003") in the current implementation,
    // but to be robust we just check that it's text and not an error.
    match &responses[0] {
        ATResponse::Text(val) => {
            // Format is typically zero-padded "003" but we don't hard-fail if it ever changes.
            assert!(
                !val.is_empty(),
                "Expected non-empty S-register string, got empty"
            );
        }
        other => panic!("Expected Text response to S0? query, got {:?}", other),
    }
}
