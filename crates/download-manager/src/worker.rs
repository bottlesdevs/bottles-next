use crate::{DownloadError, DownloadResult, Request};
use anyhow::anyhow;
use reqwest::Client;
use std::time::Duration;
use tokio::{fs::File, io::AsyncWriteExt};

pub(super) async fn download_thread(client: Client, mut request: Request) {
    request.mark_running();
    let mut last_retryable_error: Option<anyhow::Error> = None;

    let retries = request.config().retries();
    for attempt in 0..=retries {
        if attempt > 0 {
            request.mark_retrying(attempt);

            //TODO: Add proper backoff
            let mut delay = tokio::time::interval(Duration::from_secs(1));

            tokio::select! {
                _ = delay.tick() => {},
                _ = request.cancel_token.cancelled() => {
                    request.mark_cancelled();
                    return;
                }
            }
        }

        match attempt_download(client.clone(), &mut request).await {
            Ok(result) => {
                request.mark_completed(result);
                return;
            }
            Err(error) if error.is_retryable() => {
                last_retryable_error = Some(error.into());
                continue;
            }
            Err(error) => {
                request.mark_failed(error);
                return;
            }
        };
    }

    request.mark_failed(DownloadError::RetriesExhausted {
        last_error: last_retryable_error.unwrap_or_else(|| anyhow!("Unknwown Error")),
    });
}

async fn attempt_download(
    client: Client,
    request: &mut Request,
) -> Result<DownloadResult, DownloadError> {
    let mut response = client
        .get(request.url().as_ref())
        .send()
        .await?
        .error_for_status()?;

    // Create the destination directory if it doesn't exist
    if let Some(parent) = request.destination().parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    if request.destination().exists() && !request.config().overwrite() {
        return Err(DownloadError::FileExists {
            path: request.destination().to_path_buf(),
        });
    }

    let mut file = File::create(&request.destination()).await?;

    loop {
        tokio::select! {
            _ = request.cancel_token.cancelled() => {
                drop(file); // Manually drop the file handle to ensure that deletion doesn't fail
                tokio::fs::remove_file(&request.destination()).await?;
                return Err(DownloadError::Cancelled);
            }
            chunk = response.chunk() => {
                match chunk {
                    Ok(Some(chunk)) => {
                        file.write_all(&chunk).await?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        drop(file); // Manually drop the file handle to ensure that deletion doesn't fail
                        tokio::fs::remove_file(&request.destination()).await?;
                        return Err(e.into());
                    },
                }
            }
        }
    }

    // Ensure the data is written to disk
    file.sync_all().await?;

    Ok(DownloadResult {
        path: request.destination().to_path_buf(),
    })
}
