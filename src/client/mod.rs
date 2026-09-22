pub mod types;

use std::sync::Arc;

use anyhow::{Context, Result};
use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient, Json, Url};

pub use types::{Health, Session};

#[derive(Clone)]
pub struct OpencodeClient {
    http: Arc<dyn HttpClient>,
    base: Url,
}

impl OpencodeClient {
    pub fn new(base: Url, http: Arc<dyn HttpClient>) -> Self {
        Self { http, base }
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .with_context(|| format!("join {path} onto {}", self.base))
    }

    pub async fn health(&self) -> Result<Health> {
        let url = self.url("/global/health")?;
        let response = self
            .http
            .get(url.as_str(), AsyncBody::empty(), true)
            .await
            .context("GET /global/health")?;

        if !response.status().is_success() {
            anyhow::bail!("GET /global/health -> {}", response.status());
        }

        let mut body = String::new();
        response
            .into_body()
            .read_to_string(&mut body)
            .await
            .context("read health body")?;

        serde_json::from_str(&body).context("parse health JSON")
    }

    pub async fn create_session(&self, title: &str) -> Result<Session> {
        #[derive(serde::Serialize)]
        struct NewSession<'a> {
            title: &'a str,
        }

        let url = self.url("/session")?;
        let payload = NewSession { title };

        let response = self
            .http
            .post_json(url.as_str(), Json(&payload).into())
            .await
            .context("POST /session")?;

        if !response.status().is_success() {
            anyhow::bail!("POST /session -> {}", response.status());
        }

        let mut body = String::new();
        response
            .into_body()
            .read_to_string(&mut body)
            .await
            .context("read session body")?;

        serde_json::from_str(&body).context("parse session JSON")
    }
}
