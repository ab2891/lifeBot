use anyhow::{bail, Result};
use chrono::NaiveDate;

/// Validate a date string is YYYY-MM-DD format
pub fn validate_date(value: &str, field: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("{field} must be a valid date in YYYY-MM-DD format, got: {value}"))
}

/// Validate a date range (start <= end)
pub fn validate_date_range(start: &str, end: &str) -> Result<()> {
    let s = validate_date(start, "start date")?;
    let e = validate_date(end, "end date")?;
    if s > e {
        bail!("Start date ({start}) must be before or equal to end date ({end})");
    }
    Ok(())
}

/// Validate string length
pub fn validate_length(value: &str, field: &str, max: usize) -> Result<()> {
    if value.len() > max {
        bail!("{field} is too long ({} chars, max {max})", value.len());
    }
    if value.trim().is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(())
}

/// Validate a URL is safe (http/https/rtsp only, no internal IPs)
pub fn validate_url(value: &str, field: &str) -> Result<()> {
    validate_length(value, field, 2048)?;
    // Must start with allowed protocol
    let lower = value.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://")
       && !lower.starts_with("rtsp://") && !lower.starts_with("rtsps://")
       && !lower.starts_with("mock://") {
        bail!("{field} must use http, https, rtsp, rtsps, or mock protocol");
    }
    // Block internal IPs
    for blocked in &["://127.", "://localhost", "://0.0.0.0", "://10.", "://192.168.", "://172.16.", "://169.254.", "://[::1]"] {
        if lower.contains(blocked) {
            bail!("{field} cannot reference internal network addresses");
        }
    }
    Ok(())
}

/// Validate sentinel action is from allowed set
pub fn validate_sentinel_action(action: &str) -> Result<()> {
    match action {
        "acknowledged" | "dismissed" | "false_positive" | "escalated" | "resolved" => Ok(()),
        _ => bail!("Invalid sentinel action: {action}. Must be one of: acknowledged, dismissed, false_positive, escalated, resolved"),
    }
}
