use super::Request;
use crate::{DownloadError, DownloadResult};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};

pub(super) async fn download_thread(client: Client, mut request: Request) {
    request.mark_running();

    let _ = match attempt_download(client.clone(), &mut request).await {
        Ok(result) => request.mark_completed(result),
        Err(error) => match error {
            DownloadError::Cancelled => request.mark_cancelled(),
            _ => request.mark_failed(error),
        },
    };
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
