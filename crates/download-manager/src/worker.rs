use std::sync::Arc;

use reqwest::{Client, Method};
use tokio::{fs::File, io::AsyncWriteExt, sync::mpsc};
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{
    context::{Context, DownloadID},
    download::RemoteInfo,
    error::DownloadError,
    events::{Event, Progress},
    prelude::DownloadResult,
    request::Request,
};

pub(crate) enum WorkerMsg {
    Finish {
        id: DownloadID,
        result: Result<DownloadResult, DownloadError>,
    },
}

#[instrument(level = "info", skip(request, ctx, worker_tx), fields(id = %request.id(), url = %request.url()))]
pub(crate) async fn run(
    request: Arc<Request>,
    ctx: Arc<Context>,
    worker_tx: mpsc::Sender<WorkerMsg>,
) {
    let result = attempt_download(request.as_ref(), ctx.client.clone()).await;
    if result.is_ok() {
        info!(id = %request.id(), "Download attempt finished successfully");
    } else {
        warn!(id = %request.id(), "Download attempt finished with error");
    }

    let _ = worker_tx
        .send(WorkerMsg::Finish {
            id: request.id(),
            result,
        })
        .await;
}

#[instrument(level = "debug", skip(request, client), fields(id = %request.id(), url = %request.url()))]
pub(crate) async fn probe_head(request: &Request, client: &Client) -> Option<RemoteInfo> {
    use reqwest::header;
    debug!("Probing remote with HTTP HEAD");
    let req = client
        .request(Method::HEAD, request.url().as_ref())
        .headers(request.config().headers().clone())
        .send();

    let resp = tokio::select! {
        resp = req => resp.ok()?.error_for_status().ok()?,
        _ = request.cancel_token.cancelled() => return None,
    };

    let headers = resp.headers();
    let content_length = resp.content_length();
    trace!(content_length = ?content_length, "Got HEAD response");
    let accept_ranges = headers
        .get(header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let etag = headers
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = headers
        .get(header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    Some(RemoteInfo {
        content_length,
        accept_ranges,
        etag,
        last_modified,
        content_type,
    })
}

#[instrument(level = "info", skip(request, client), fields(id = %request.id(), url = %request.url(), destination = ?request.destination()))]
pub(crate) async fn attempt_download(
    request: &Request,
    client: Client,
) -> Result<DownloadResult, DownloadError> {
    if let Some(info) = probe_head(request, &client).await {
        request.emit(Event::Probed {
            id: request.id(),
            info,
        });
    }

    if let Some(parent) = request.destination().parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if request.destination().exists() && !request.config().overwrite() {
        warn!(destination = ?request.destination(), "Destination exists and overwrite=false; failing");
        return Err(DownloadError::FileExists {
            path: request.destination().to_path_buf(),
        });
    }

    let req = client
        .request(Method::GET, request.url().as_ref())
        .headers(request.config().headers().clone())
        .send();

    let mut response = tokio::select! {
      resp = req => Ok(resp?.error_for_status()?),
        _ = request.cancel_token.cancelled() =>  Err(DownloadError::Cancelled),
    }?;
    let total_bytes = response.content_length();
    debug!(total_bytes = ?total_bytes, "Server accepted download");

    let mut file = File::create(request.destination()).await?;
    request.emit(Event::Started {
        id: request.id(),
        url: request.url().clone(),
        destination: request.destination().to_path_buf(),
        total_bytes,
    });

    let mut progress = Progress::new(total_bytes);
    loop {
        tokio::select! {
            _ = request.cancel_token.cancelled() => {
                warn!(destination = ?request.destination(), "Cancellation received; cleaning up partial file");
                drop(file);
                tokio::fs::remove_file(request.destination()).await?;
                return Err(DownloadError::Cancelled);
            }
            chunk = response.chunk() => {
                match chunk {
                    Ok(Some(chunk)) => {
                        file.write_all(&chunk).await?;
                        if progress.update(chunk.len() as u64) {
                            request.update_progress(progress);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!(error = %e, destination = ?request.destination(), "Error while reading response chunk; removing partial file");
                        drop(file);
                        tokio::fs::remove_file(request.destination()).await?;
                        return Err(e.into());
                    }
                }
            }
        }
    }

    progress.force_update();
    let _ = request.update_progress(progress);
    file.sync_all().await?;
    info!(destination = ?request.destination(), bytes = progress.bytes_downloaded(), "Download completed successfully");

    Ok(DownloadResult {
        path: request.destination().to_path_buf(),
        bytes_downloaded: progress.bytes_downloaded(),
    })
}
