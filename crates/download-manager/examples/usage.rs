use std::path::PathBuf;

use download_manager::{DownloadManager, prelude::*};
use futures_util::StreamExt;
use reqwest::Url;
// use std::fmt::Debug;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    // Configure logs via RUST_LOG if provided, else use a sensible default.
    if std::env::var_os("RUST_LOG").is_none() {
        // Show info logs globally and debug logs for this crate
        unsafe {
            std::env::set_var("RUST_LOG", "info,download_manager=debug");
        }
    }

    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let manager = DownloadManager::default();

    let url = Url::parse("https://ash-speed.hetzner.com/100MB.bin")?;
    let destination: PathBuf = "example-download.bin".into();

    // Start the download
    let download = manager.download(url, &destination)?;

    // Subscribe to per-download progress
    let mut progress_stream = download.progress();
    tokio::spawn(async move {
        while let Some(p) = progress_stream.next().await {
            let pct = p
                .percent()
                .map(|v| format!("{v:.1}%"))
                .unwrap_or_else(|| "?".into());
            info!(
                bytes = p.bytes_downloaded,
                total = p.total_bytes.map(|v| v as i64).unwrap_or(-1),
                instantaneous_bps = (p.instantaneous_bps as u64),
                ema_bps = (p.ema_bps as u64),
                percent = %pct,
                "progress"
            );
        }
    });

    // Subscribe to per-download events
    let mut event_stream = download.events();
    tokio::spawn(async move {
        while let Some(ev) = event_stream.next().await {
            // Event implements Display; we also log structured data above via tracing in the library.
            info!(event = %ev, "event");
        }
    });

    // Optionally, you can also subscribe to global events across all downloads:
    // let mut global_events = manager.events();
    // tokio::spawn(async move {
    //     while let Some(ev) = global_events.next().await {
    //         info!(event = %ev, "global_event");
    //     }
    // });

    // Await the result (the handle implements Future)
    match download.await {
        Ok(result) => {
            info!(
                path = %result.path.display(),
                bytes = result.bytes_downloaded,
                "download completed"
            );
        }
        Err(err) => {
            error!(error = %err, "download failed");
        }
    }

    // Graceful shutdown (waits for any background tasks to finish)
    manager.shutdown().await;

    Ok(())
    // To test cancellation, you could do:
    // let id = download.id();
    // tokio::spawn({
    //     let manager = manager.clone();
    //     async move {
    //         tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    //         let _ = manager.cancel(id).await;
    //     }
    // });
}
