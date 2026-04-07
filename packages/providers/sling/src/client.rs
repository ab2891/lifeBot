use anyhow::{bail, Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::types::*;

const BASE_URL: &str = "https://api.getsling.com/v1";

pub struct SlingClient {
    http: reqwest::Client,
    token: String,
    org_id: i64,
}

impl SlingClient {
    /// Authenticate with email + password. Returns a ready-to-use client.
    pub async fn login(email: &str, password: &str) -> Result<Self> {
        let http = reqwest::Client::new();

        let login_url = format!("{}/account/login", BASE_URL);
        let body = serde_json::json!({ "email": email, "password": password });

        let resp = http
            .post(&login_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to send login request")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("login failed with status {}: {}", status, text);
        }

        let token = resp
            .headers()
            .get(AUTHORIZATION)
            .context("Authorization header missing from login response")?
            .to_str()
            .context("Authorization header is not valid UTF-8")?
            .to_string();

        // Fetch org_id from session
        let session_url = format!("{}/account/session", BASE_URL);
        let session_resp = http
            .get(&session_url)
            .header(AUTHORIZATION, &token)
            .send()
            .await
            .context("failed to send session request")?;

        let session_status = session_resp.status();
        if !session_status.is_success() {
            let text = session_resp.text().await.unwrap_or_default();
            bail!("session request failed with status {}: {}", session_status, text);
        }

        let session: serde_json::Value = session_resp
            .json()
            .await
            .context("failed to parse session response")?;

        let org_id = session["orgs"][0]["id"]
            .as_i64()
            .or_else(|| session["org"]["id"].as_i64())
            .or_else(|| session["orgId"].as_i64())
            .context("could not find org_id in session response")?;

        Ok(Self { http, token, org_id })
    }

    /// Reconnect using a previously stored token and org_id.
    pub fn from_token(token: String, org_id: i64) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
            org_id,
        }
    }

    /// Returns the organisation ID.
    pub fn org_id(&self) -> i64 {
        self.org_id
    }

    /// Returns the raw auth token.
    pub fn token(&self) -> &str {
        &self.token
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", BASE_URL, path);
        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .with_context(|| format!("GET {} failed", url))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("GET {} returned {}: {}", url, status, text);
        }

        resp.json::<serde_json::Value>()
            .await
            .with_context(|| format!("failed to parse JSON from GET {}", url))
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", BASE_URL, path);
        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, &self.token)
            .header(CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {} failed", url))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("POST {} returned {}: {}", url, status, text);
        }

        resp.json::<serde_json::Value>()
            .await
            .with_context(|| format!("failed to parse JSON from POST {}", url))
    }

    // -----------------------------------------------------------------------
    // Public API methods
    // -----------------------------------------------------------------------

    /// Fetch all non-deleted users (concise endpoint).
    pub async fn fetch_users(&self) -> Result<Vec<SlingUser>> {
        let val = self.get("/users/concise").await?;
        let users: Vec<SlingUser> = serde_json::from_value(val)
            .context("failed to deserialize users")?;
        Ok(users.into_iter().filter(|u| !u.deleted).collect())
    }

    /// Fetch all groups (locations and positions).
    pub async fn fetch_groups(&self) -> Result<Vec<SlingGroup>> {
        let val = self.get("/groups").await?;
        serde_json::from_value(val).context("failed to deserialize groups")
    }

    /// Fetch shifts for the given ISO 8601 interval, e.g.
    /// `"2026-03-01T00:00:00Z/2026-03-31T23:59:59Z"`.
    pub async fn fetch_shifts(&self, dates: &str) -> Result<Vec<SlingShift>> {
        let path = format!("/calendar/{}/users/0?dates={}", self.org_id, dates);
        let val = self.get(&path).await?;
        serde_json::from_value(val).context("failed to deserialize shifts")
    }

    /// Bulk-create shifts.
    pub async fn create_shifts(&self, shifts: &[SlingShiftCreate]) -> Result<serde_json::Value> {
        let body = serde_json::to_value(shifts).context("failed to serialize shifts")?;
        self.post("/shifts/bulk", &body).await
    }

    /// Health check — returns true if the session endpoint responds successfully.
    pub async fn health(&self) -> Result<bool> {
        match self.get("/account/session").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
