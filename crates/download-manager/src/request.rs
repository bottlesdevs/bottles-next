use crate::{Download, DownloadManager};
use anyhow::Result;
use reqwest::Url;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Request {
    url: Url,
    destination: PathBuf,
}

impl Request {
    pub fn builder<'a>(manager: &'a DownloadManager) -> RequestBuilder<'a> {
        RequestBuilder {
            url: None,
            destination: None,
            manager,
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn destination(&self) -> &Path {
        self.destination.as_path()
    }
}

pub struct RequestBuilder<'a> {
    url: Option<Url>,
    destination: Option<PathBuf>,

    manager: &'a DownloadManager,
}

impl RequestBuilder<'_> {
    pub fn url(mut self, url: Url) -> Self {
        self.url = Some(url);
        self
    }

    pub fn destination(mut self, destination: impl AsRef<Path>) -> Self {
        self.destination = Some(destination.as_ref().to_path_buf());
        self
    }

    pub fn start(self) -> Result<Download> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;

        let request = Request { url, destination };

        self.manager.queue_request(request)?;

        Ok(Download::new())
    }
}
