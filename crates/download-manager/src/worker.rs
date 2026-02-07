use std::{io::SeekFrom, sync::Arc};

use reqwest::{Client, Method, StatusCode, header};
use tokio::{
    fs::OpenOptions,
    io::{AsyncSeekExt, AsyncWriteExt, BufWriter},
    sync::mpsc,
};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{
    context::{Context, DownloadID},
    download::{CacheMeta, RemoteInfo},
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

#[instrument(level = "debug", skip(request, client), fields(id = %request.id(), url = %request.url(), method = "HEAD"))]
pub(crate) async fn probe_head(request: &Request, client: &Client) -> Option<RemoteInfo> {
    trace!("starting HEAD probe");

    let head_req = client
        .request(Method::HEAD, request.url().as_ref())
        .headers(request.config().headers().clone())
        .send();

    let resp = tokio::select! {
        resp = head_req => {
            match resp {
                Ok(r) => match r.error_for_status() {
                    Ok(ok) => Some(ok),
                    Err(e) => {
                        debug!(error = %e, "HEAD returned error status");
                        None
                    }
                },
                Err(e) => {
                    debug!(error = %e, "HEAD request failed");
                    None
                }
            }
        }
        _ = request.cancel_token.cancelled() => {
            debug!("probe cancelled during HEAD");
            return None;
        }
        _ = request.pause_token.cancelled() => {
            debug!("probe paused during HEAD");
            // Treat a pause during probe as a non-error probe result; upper layers will handle paused semantics.
            return None;
        }
    };

    let resp = match resp {
        Some(r) => r,
        None => {
            debug!("HEAD probe unusable → falling back to RANGE probe");
            return probe_range(request, client).await;
        }
    };

    let info = extract_info(&resp);

    if info.content_length.is_none() && info.accept_ranges.is_none() {
        debug!("HEAD missing critical headers → fallback to RANGE probe");
        return probe_range(request, client).await;
    }

    debug!("HEAD probe successful");
    Some(info)
}

#[instrument(level = "debug", skip(request, client), fields(id = %request.id(), url = %request.url(), method = "RANGE_GET"))]
async fn probe_range(request: &Request, client: &Client) -> Option<RemoteInfo> {
    use reqwest::header;

    trace!("starting Range GET probe (bytes=0-0)");

    let resp = client
        .get(request.url().as_ref())
        .header(header::RANGE, "bytes=0-0")
        .headers(request.config().headers().clone())
        .send()
        .await;

    let resp = match resp {
        Ok(r) => match r.error_for_status() {
            Ok(ok) => ok,
            Err(e) => {
                debug!(error = %e, "Range probe returned error status");
                return None;
            }
        },
        Err(e) => {
            debug!(error = %e, "Range probe request failed");
            return None;
        }
    };

    debug!(
        status = %resp.status(),
        content_range = ?resp.headers().get(header::CONTENT_RANGE),
        "Range probe response received"
    );

    Some(extract_info(&resp))
}

#[instrument(level = "trace", skip(resp))]
fn extract_info(resp: &reqwest::Response) -> RemoteInfo {
    use reqwest::header;

    let headers = resp.headers();

    let content_length = headers
        .get(header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').nth(1))
        .and_then(|total| total.parse::<u64>().ok())
        .or_else(|| resp.content_length());

    RemoteInfo {
        content_length,
        accept_ranges: headers
            .get(header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        meta: CacheMeta {
            etag: headers
                .get(header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            last_modified: headers
                .get(header::LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        },
        content_type: headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    }
}

#[instrument(level = "info", skip(request, client), fields(id = %request.id(), url = %request.url(), destination = ?request.destination()))]
pub(crate) async fn attempt_download(
    request: &Request,
    client: Client,
) -> Result<DownloadResult, DownloadError> {
    let path = request.destination();

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved_meta = CacheMeta::load(path).await;

    let existing_len = match tokio::fs::metadata(path).await {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    let mut resuming = existing_len > 0 && saved_meta.is_some();

    if let Some(info) = probe_head(request, &client).await {
        info.meta.save(path).await?;
        request.emit(Event::Probed {
            id: request.id(),
            info,
        });
    }

    if existing_len > 0 && saved_meta.is_none() && !request.config().overwrite() {
        warn!(destination = ?path, "Destination exists and overwrite=false; failing");
        return Err(DownloadError::FileExists {
            path: path.to_path_buf(),
        });
    }

    let mut rb = client.request(Method::GET, request.url().as_ref());

    for (k, v) in request.config().headers().iter() {
        rb = rb.header(k, v);
    }

    if let Some(meta) = saved_meta {
        rb = rb.header(header::RANGE, format!("bytes={}-", existing_len));

        if let Some(etag) = meta.etag {
            rb = rb.header(header::IF_RANGE, etag);
        } else if let Some(last_modified) = meta.last_modified {
            rb = rb.header(header::IF_RANGE, last_modified);
        }

        rb = rb.header(header::ACCEPT_ENCODING, "identity");
    }

    let req = rb.send();

    let response = tokio::select! {
        resp = req => match resp {
            Ok(rsp) => rsp,
            Err(err) => return Err(err.into()),
        },
        _ = request.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
        _ = request.pause_token.cancelled() => return Err(DownloadError::Paused),
    };

    let total_bytes = response
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').nth(1))
        .and_then(|t| t.parse::<u64>().ok())
        .or_else(|| response.content_length());

    debug!(total_bytes = ?total_bytes, status = %response.status(), "Server accepted download");

    if resuming && response.status() != StatusCode::PARTIAL_CONTENT {
        warn!(status = %response.status(), "Resume failed: server did not return 206");
        resuming = false;

        if !request.config().overwrite() {
            return Err(DownloadError::FileExists {
                path: path.to_path_buf(),
            });
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resuming)
        .truncate(!resuming)
        .open(path)
        .await?;

    let mut writer = BufWriter::with_capacity(256 * 256, file);

    if resuming {
        writer.seek(SeekFrom::End(0)).await?;
    }

    request.emit(Event::Started {
        id: request.id(),
        url: request.url().clone(),
        destination: path.to_path_buf(),
        total_bytes,
    });

    let mut progress = Progress::new(total_bytes);

    if resuming {
        progress = progress.resume(existing_len);
    }

    let mut stream = response.bytes_stream();

    loop {
        tokio::select! {
            _ = request.cancel_token.cancelled() => {
                warn!("Cancellation received; cleaning up");
                // Remove both the partially downloaded file and its metadata file if present.
                let _ = tokio::fs::remove_file(path).await;
                let _ = tokio::fs::remove_file(CacheMeta::meta_path(path)).await;
                return Err(DownloadError::Cancelled);
            }
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(chunk)) => {
                        // Responsive pause: check the pause token synchronously before writing the chunk.
                        // This avoids waiting for the next iteration of the select! to notice the pause.
                        if request.pause_token.is_cancelled() {
                            warn!("Pause received while writing chunk; leaving partial file intact for potential resume");
                            return Err(DownloadError::Paused);
                        }
                        writer.write_all(&chunk).await?;
                        if progress.update(chunk.len() as u64) {
                            request.update_progress(progress);
                        }
                    }
                    None => break,
                    Some(Err(e)) => {
                        error!(error = %e, "Chunk read failed; removing partial file");
                        // Best-effort cleanup: remove the partial download and its metadata file, but
                        // don't let cleanup errors mask the original chunk read error.
                        let _ = tokio::fs::remove_file(path).await;
                        let _ = tokio::fs::remove_file(CacheMeta::meta_path(path)).await;
                        return Err(e.into());
                    }
                }
            }
            _ = request.pause_token.cancelled() => {
                warn!("Pause received; leaving partial file for resume");
                return Err(DownloadError::Paused);
            }
        }
    }

    progress.force_update();
    request.update_progress(progress);

    writer.flush().await?;
    let file = writer.into_inner();

    file.sync_all().await?;
    tokio::fs::remove_file(CacheMeta::meta_path(path)).await?;

    info!(
        destination = ?path,
        bytes = progress.bytes_downloaded(),
        "Download completed successfully"
    );

    Ok(DownloadResult {
        path: path.to_path_buf(),
        bytes_downloaded: progress.bytes_downloaded(),
    })
}
