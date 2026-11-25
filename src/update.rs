//! "Is there a newer KO II OS?" Reads TE's public "what's new" page (nothing is sent but a
//! normal page request) and finds the highest "os X.Y" it mentions.
//! The app never installs firmware itself: the update is done with TE's own update page.

/// TE's list of KO II OS releases.
pub const WHATS_NEW_URL: &str = "https://teenage.engineering/guides/ep-133/whats-new";
/// TE's official update tool (runs in Chrome or Edge).
pub const UPDATE_URL: &str = "https://teenage.engineering/apps/update";
/// TE's official EP Sample Tool: backup, restore, sounds.
pub const SAMPLE_TOOL_URL: &str = "https://teenage.engineering/apps/ep-sample-tool";

/// "2.5.1" -> [2, 5, 1].
pub fn parse_version(text: &str) -> Option<Vec<u32>> {
    let parts: Option<Vec<u32>> = text.trim().split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

/// Is `latest` newer than `installed`? Missing parts count as 0 (2.5 = 2.5.0).
pub fn is_newer(latest: &str, installed: &str) -> bool {
    let (Some(mut a), Some(mut b)) = (parse_version(latest), parse_version(installed)) else {
        return false;
    };
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a > b
}

/// Highest "os X.Y" or "os X.Y.Z" on the page (upper or lower case).
pub fn newest_on_page(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut best: Option<(Vec<u32>, String)> = None;
    for (i, _) in lower.match_indices("os ") {
        let rest = &lower[i + 3..];
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        let found = rest[..end].trim_end_matches('.');
        if !found.contains('.') {
            continue;
        }
        if let Some(v) = parse_version(found) {
            if best.as_ref().is_none_or(|(b, _)| v > *b) {
                best = Some((v, found.to_string()));
            }
        }
    }
    best.map(|(_, text)| text)
}

/// Download TE's page and return the newest OS it lists. Runs on a background thread.
pub fn fetch_latest() -> Result<String, String> {
    let mut response = ureq::get(WHATS_NEW_URL)
        .header("User-Agent", "KO II Buddy (desktop app)")
        .call()
        .map_err(|e| format!("Could not reach teenage.engineering ({e})."))?;
    let html = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Could not read TE's page ({e})."))?;
    newest_on_page(&html).ok_or_else(|| "TE's page did not show an OS version.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("2.6", "2.5.1"));
        assert!(is_newer("2.5.2", "2.5.1"));
        assert!(!is_newer("2.5", "2.5.1"));
        assert!(!is_newer("2.5.1", "2.5.1"));
        assert!(!is_newer("abc", "2.5.1"));
    }

    #[test]
    fn finds_newest_os_on_page() {
        // Same shapes as TE's page on 2026-11-25.
        let html = r#"<a href="/guides/ep-133/whats-new#2.0.2">OS 2.0.2</a> new OS 2.5 features <span>os 2.0.5</span> os 2.0.1"#;
        assert_eq!(newest_on_page(html).as_deref(), Some("2.5"));
        assert_eq!(newest_on_page("no versions here"), None);
    }
}
