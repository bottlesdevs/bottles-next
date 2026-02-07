use std::path::PathBuf;
use std::time::Duration;

use download_manager::{DownloadManager, prelude::*};
use futures_util::StreamExt;
use reqwest::Url;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    if std::env::var_os("RUST_LOG").is_none() {
        // Sensible default if RUST_LOG is not set in the environment.
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

    // Example file and URL: replace with something suitable for testing.
    // The server should support range requests for resume to work reliably.
    let url = Url::parse("https://ash-speed.hetzner.com/100MB.bin")?;
    let destination: PathBuf = "example-download.bin".into();

    let download = manager.download(url, &destination)?;
    let id = download.id();

    // Subscribe to progress updates from the `Download` handle.
    //
    // `progress()` returns a stream of sampled `Progress` values (watch channel). Consumers
    // receive the latest value immediately and then updates as they are emitted.
    // We spawn a background task to log progress so the main task can focus on await/shutdown.
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
        info!("progress stream ended for id={}", id);
    });

    // Subscribe to per-download events (Queued, Started, Paused, Completed, Failed, etc.).
    //
    // `events()` yields a stream filtered to this download's events. These are the same events
    // that are also broadcast globally by the manager; here we log them for visibility.
    let mut events = download.events();
    tokio::spawn(async move {
        while let Some(ev) = events.next().await {
            info!(event = %ev, "download event");
        }
        info!("events stream ended for id={}", id);
    });

    // Demonstrate pausing and resuming via the manager.
    //
    // Behavior:
    // - We wait a short time (so the download starts and makes progress), then call `manager.pause(id)`.
    // - The scheduler will attempt to pause the active worker; when the worker cooperatively returns
    //   a `Paused` error the scheduler preserves the job state and partial file for later resume.
    // - After a further delay we call `manager.resume(id)`, which re-enqueues the job and resumes
    //   downloading from the existing partial file (if the server supports Range requests).
    //
    // Note: the `Download` handle's original progress/event streams remain valid across pause/resume.
    {
        let manager = manager.clone();
        tokio::spawn(async move {
            // Wait so the download has time to start and produce progress updates.
            tokio::time::sleep(Duration::from_secs(5)).await;

            info!("Requesting pause for id={}", id);
            if let Err(e) = manager.pause(id).await {
                error!(error = %e, "pause request failed");
                return;
            }

            // Simulate user delay while paused.
            tokio::time::sleep(Duration::from_secs(3)).await;

            info!("Requesting resume for id={}", id);
            if let Err(e) = manager.resume(id).await {
                error!(error = %e, "resume request failed");
            }
        });
    }

    // Await the final result from the `Download` future.
    //
    // The future will complete with:
    // - `Ok(DownloadResult)` when the download finishes successfully,
    // - `Err(DownloadError::Cancelled)` if cancelled, or
    // - `Err(DownloadError::Paused)` if paused and the caller chose to treat pause as terminal.
    // In this example we expect the resume to cause eventual success (if the server supports ranges).
    match download.await {
        Ok(result) => {
            info!(
                path = %result.path.display(),
                bytes = result.bytes_downloaded,
                "download completed"
            );
        }
        Err(err) => {
            error!(error = %err, "download failed or was interrupted");
        }
    }

    // Gracefully shut down the manager. This cancels any remaining activity and waits for
    // worker tasks to finish. Always call this when you want deterministic teardown.
    manager.shutdown().await;
    Ok(())
}
