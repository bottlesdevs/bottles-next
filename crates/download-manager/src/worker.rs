use super::Request;
use crate::DownloadResult;
use anyhow::{Result, anyhow};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};

pub(super) async fn download_thread(client: Client, mut request: Request) {
    request.mark_running();
    match attempt_download(client.clone(), &mut request).await {
        Ok(result) => {
            request.mark_completed();
            // TODO: Send the result to the user
        }
        Err(e) => {
            request.mark_failed();
            // TODO: Try to retry or fail
        }
    }
}

async fn attempt_download(client: Client, request: &mut Request) -> Result<DownloadResult> {
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
            chunk = response.chunk() => {
                match chunk {
                    Ok(Some(chunk)) => {
                        file.write_all(&chunk).await?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        drop(file); // Manually drop the file handle to ensure that deletion doesn't fail
                        tokio::fs::remove_file(&request.destination()).await?;
                        return Err(anyhow!(e));
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
