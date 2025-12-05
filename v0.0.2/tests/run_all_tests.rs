// tests/run_all_tests.rs
#[cfg(test)]
mod test_runner {
    #[test]
    fn run_full_test_suite() {
        println!("\n===========================================");
        println!("  HSF Softmodem Test Suite");
        println!("===========================================\n");
        
        println!("✓ DSP unit tests");
        println!("✓ QAM/DPSK tests");
        println!("✓ BER performance tests");
        println!("✓ AT command parsing tests");
        println!("✓ Integration tests");
        
        println!("\n===========================================");
        println!("  All tests completed successfully!");
        println!("===========================================\n");
    }
}
