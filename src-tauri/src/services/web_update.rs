use std::cmp::Ordering;

const RELEASES_API_BASE_URL_ENV: &str = "CC_SWITCH_RELEASES_API_BASE_URL";
const DEFAULT_RELEASES_API_BASE_URL: &str = "https://api.github.com";
const RELEASES_API_PATH: &str = "/repos/farion1231/cc-switch/releases/latest";

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebUpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GithubLatestRelease {
    tag_name: Option<String>,
    body: Option<String>,
    html_url: Option<String>,
}

pub async fn get_web_update_info() -> WebUpdateInfo {
    let client = crate::proxy::http_client::get();
    let response = match client
        .get(latest_release_url())
        .header("User-Agent", "cc-switch")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return WebUpdateInfo::unavailable(),
    };

    if !response.status().is_success() {
        return WebUpdateInfo::unavailable();
    }

    let release = match response.json::<GithubLatestRelease>().await {
        Ok(release) => release,
        Err(_) => return WebUpdateInfo::unavailable(),
    };

    let version = release.tag_name.as_deref().and_then(normalize_version);
    let available = version
        .as_deref()
        .map(|latest| is_update_available(env!("CARGO_PKG_VERSION"), latest))
        .unwrap_or(false);

    WebUpdateInfo {
        available,
        version,
        notes: normalize_optional_text(release.body),
        download_url: normalize_optional_text(release.html_url),
    }
}

impl WebUpdateInfo {
    fn unavailable() -> Self {
        Self {
            available: false,
            version: None,
            notes: None,
            download_url: None,
        }
    }
}

fn latest_release_url() -> String {
    format!("{}{}", releases_api_base_url(), RELEASES_API_PATH)
}

fn releases_api_base_url() -> String {
    std::env::var(RELEASES_API_BASE_URL_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_API_BASE_URL.to_string())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|inner| {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_version(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_prefix = trimmed.strip_prefix('v').unwrap_or(trimmed).trim();
    if without_prefix.is_empty() {
        None
    } else {
        Some(without_prefix.to_string())
    }
}

fn is_update_available(current: &str, latest: &str) -> bool {
    let normalized_current = match normalize_version(current) {
        Some(value) => value,
        None => return false,
    };

    let normalized_latest = match normalize_version(latest) {
        Some(value) => value,
        None => return false,
    };

    match (
        parse_semverish_version(&normalized_current),
        parse_semverish_version(&normalized_latest),
    ) {
        (Some(current_version), Some(latest_version)) => latest_version > current_version,
        _ => normalized_latest != normalized_current,
    }
}

fn parse_semverish_version(value: &str) -> Option<SemverishVersion> {
    let mut parts = value.splitn(2, '-');
    let core = parts.next()?;
    let prerelease = parts.next().map(|part| part.to_string());
    let mut numbers = core.split('.');

    let major = numbers.next()?.parse::<u64>().ok()?;
    let minor = numbers.next()?.parse::<u64>().ok()?;
    let patch = numbers.next()?.parse::<u64>().ok()?;

    if numbers.next().is_some() {
        return None;
    }

    Some(SemverishVersion {
        major,
        minor,
        patch,
        prerelease,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemverishVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Option<String>,
}

impl Ord for SemverishVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let core_cmp = (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core_cmp != Ordering::Equal {
            return core_cmp;
        }

        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for SemverishVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_update_available, latest_release_url, normalize_optional_text, normalize_version,
        parse_semverish_version,
    };

    #[test]
    fn normalizes_versions_and_optional_text() {
        assert_eq!(normalize_version("v3.15.0"), Some("3.15.0".to_string()));
        assert_eq!(normalize_version(" 3.15.0-beta.1 "), Some("3.15.0-beta.1".to_string()));
        assert_eq!(normalize_version(""), None);
        assert_eq!(normalize_optional_text(Some("  notes  ".to_string())), Some("notes".to_string()));
        assert_eq!(normalize_optional_text(Some("   ".to_string())), None);
    }

    #[test]
    fn compares_semverish_versions() {
        assert!(is_update_available("3.14.1", "3.15.0"));
        assert!(!is_update_available("3.14.1", "3.14.1"));
        assert!(!is_update_available("3.14.2", "3.14.1"));
        assert!(is_update_available("3.14.1-beta.1", "3.14.1"));
        assert!(!is_update_available("3.14.1", "3.14.1-beta.1"));

        let parsed = parse_semverish_version("3.15.0-beta.1").expect("expected version");
        assert_eq!(parsed.major, 3);
        assert_eq!(parsed.minor, 15);
        assert_eq!(parsed.patch, 0);
        assert_eq!(parsed.prerelease.as_deref(), Some("beta.1"));
    }

    #[test]
    fn honors_base_url_override_for_release_api() {
        let original = std::env::var("CC_SWITCH_RELEASES_API_BASE_URL").ok();
        std::env::set_var("CC_SWITCH_RELEASES_API_BASE_URL", "http://127.0.0.1:43100/");

        let actual = latest_release_url();

        if let Some(value) = original {
            std::env::set_var("CC_SWITCH_RELEASES_API_BASE_URL", value);
        } else {
            std::env::remove_var("CC_SWITCH_RELEASES_API_BASE_URL");
        }

        assert_eq!(
            actual,
            "http://127.0.0.1:43100/repos/farion1231/cc-switch/releases/latest"
        );
    }
}
