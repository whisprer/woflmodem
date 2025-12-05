// src/main.rs
mod audio;
mod dsp;
mod tapi;

use tapi::pipe_server::ModemPipeServer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to initialize logging, but don't crash if it's already initialized.
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default()
            // If RUST_LOG isn't set, default to something helpful.
            .default_filter_or("hsf_softmodem=debug"),
    )
    .try_init();

    log::info!("HSF Softmodem v1.0 - Rust Implementation");
    log::info!("Supports: Bell 103 (300 baud), V.22 (1200 bps), V.22bis (2400 bps)");

    log::debug!("Step 1/2: creating ModemPipeServer...");
    let mut server = match ModemPipeServer::new() {
        Ok(s) => {
            log::debug!("Step 1/2: ModemPipeServer created OK.");
            s
        }
        Err(e) => {
            // This is the key line we're after.
            log::error!("Step 1/2 FAILED: ModemPipeServer::new() error: {e:?}");
            return Err(Box::new(e));
        }
    };

    log::debug!("Step 2/2: entering ModemPipeServer run loop...");
    if let Err(e) = server.run() {
        log::error!("Step 2/2 FAILED: ModemPipeServer::run() error: {e:?}");
        return Err(Box::new(e));
    }

    log::debug!("Step 2/2: run loop ended cleanly.");
    Ok(())
}
