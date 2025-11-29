// src/main.rs
mod audio;
mod dsp;
mod tapi;

use tapi::pipe_server::ModemPipeServer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    log::info!("HSF Softmodem v1.0 - Rust Implementation");
    log::info!("Supports: Bell 103 (300 baud), V.22 (1200 bps), V.22bis (2400 bps)");
    
    let mut server = ModemPipeServer::new()?;
    server.run()?;
    
    Ok(())
}
