//! Shared, tauri-free IP-range classification for outbound SSRF protection.
//!
//! These helpers decide whether an IP address falls in a range the server must
//! never dial outbound (loopback / link-local / private / ULA / unspecified /
//! CGNAT). They live in the proxy tree (shared desktop + web) so BOTH the
//! web-only request guard (`web_api/handlers/common.rs::validate_outbound_url`,
//! which re-exports them) AND the web-outbound redirect policy in
//! `proxy/http_client.rs` can call the SAME classification without leaking
//! web-only code into the desktop proxy hot path.
//!
//! Note: the classification helpers (`is_blocked_*`) never resolve DNS and
//! never perform IO, so they are safe to call from a sync redirect policy
//! callback. The async `guard_outbound_url` helper DOES resolve DNS (via
//! non-blocking `tokio::net::lookup_host`) and is the single tauri-free source
//! of truth shared by the web request guard
//! (`web_api/handlers/common.rs::validate_outbound_url`, which delegates to it)
//! and the web-only usage-script SSRF guard (audit FIX 1).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

/// Returns true if the IPv4 address falls in a range the server must never dial
/// outbound: loopback (127.0.0.0/8), link-local (169.254.0.0/16), any private
/// range (10/8, 172.16/12, 192.168/16), the unspecified address (`0.0.0.0`,
/// which routes to localhost on Linux), or the CGNAT shared range
/// (100.64.0.0/10, RFC 6598 — also Tailscale's address space).
///
/// Defense-in-depth (audit FIX 8): also blocks the rest of `0.0.0.0/8` (RFC
/// 1122 "this network", non-`0.0.0.0` hosts that some stacks treat as local),
/// multicast (224.0.0.0/4), and reserved/future (240.0.0.0/4).
pub fn is_blocked_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_link_local()
        || ip.is_private()
        || ip.is_unspecified()
        || is_cgnat_ipv4(ip)
        || ip.octets()[0] == 0 // 0.0.0.0/8 (RFC 1122 "this network")
        || ip.is_multicast() // 224.0.0.0/4
        || ip.octets()[0] >= 240 // 240.0.0.0/4 reserved / 255.255.255.255 broadcast
}

/// CGNAT / RFC 6598 shared address space: 100.64.0.0/10.
fn is_cgnat_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0xc0) == 64
}

/// Returns true if the IPv6 address falls in a blocked range: loopback (::1),
/// the unspecified address (`::`, which routes to localhost on Linux),
/// link-local (fe80::/10) or unique-local / ULA (fc00::/7). The `is_*` helpers
/// for the latter two are unstable on stable Rust, so they are checked manually.
///
/// Defense-in-depth (audit FIX 8): also unwraps IPv4-compatible IPv6
/// (`::a.b.c.d`, the deprecated form `to_ipv4_mapped` misses) via `to_ipv4()`,
/// and blocks 6to4 (2002::/16), NAT64 (64:ff9b::/96), Teredo (2001::/32) and
/// multicast (ff00::/8).
pub fn is_blocked_ipv6(ip: &Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    // Catch IPv4-compatible (`::a.b.c.d`) and IPv4-mapped (`::ffff:a.b.c.d`)
    // embeddings of a blocked v4 address. `to_ipv4_mapped()` only matches the
    // mapped form, so the broader `to_ipv4()` is used here for the compat form.
    if let Some(v4) = ip.to_ipv4() {
        // `to_ipv4()` also returns Some for tiny addresses like `::1`/`::` that
        // we already handled above; for everything else, classify as v4.
        if is_blocked_ipv4(&v4) {
            return true;
        }
    }
    let segments = ip.segments();
    // Link-local fe80::/10
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // Unique-local / ULA fc00::/7
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // 6to4 2002::/16 (embeds a public v4, but can encode internal targets).
    if segments[0] == 0x2002 {
        return true;
    }
    // Teredo 2001::/32 (tunneling; can reach internal hosts).
    if segments[0] == 0x2001 && segments[1] == 0x0000 {
        return true;
    }
    // NAT64 well-known prefix 64:ff9b::/96.
    if segments[0] == 0x0064
        && segments[1] == 0xff9b
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        return true;
    }
    // Multicast ff00::/8.
    if (segments[0] & 0xff00) == 0xff00 {
        return true;
    }
    false
}

/// Returns true if the IP address is in any range disallowed for outbound
/// requests from the server. IPv4-mapped IPv6 addresses are unwrapped so a
/// mapped private/loopback/unspecified v4 (e.g. `::ffff:0.0.0.0`) cannot slip
/// through the v6 path.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(&v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_blocked_ipv4(&v4),
            None => is_blocked_ipv6(&v6),
        },
    }
}

/// Env var holding a comma-separated allow-list of hostnames that bypass the
/// SSRF guard (e.g. an internal endpoint the operator deliberately exposes).
const SSRF_ALLOW_ENV: &str = "CC_SWITCH_WEB_SSRF_ALLOW";

/// Returns true if `host` is present (case-insensitively) in the
/// `CC_SWITCH_WEB_SSRF_ALLOW` env allow-list.
pub fn ssrf_host_allowed(host: &str) -> bool {
    match std::env::var(SSRF_ALLOW_ENV) {
        Ok(list) => list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry.eq_ignore_ascii_case(host)),
        Err(_) => false,
    }
}

/// Outcome of an outbound-URL SSRF check (tauri-free, runtime-neutral). The
/// caller maps this into its own error type (`ApiError` for the web request
/// guard, `AppError` for the usage-script guard).
#[derive(Debug)]
pub enum OutboundUrlError {
    /// `raw` did not parse as a URL (`reason` is the parse error).
    InvalidUrl { raw: String, reason: String },
    /// Scheme is not http/https.
    UnsupportedScheme { scheme: String },
    /// URL had no host component.
    MissingHost { raw: String },
    /// DNS resolution of `host` failed.
    ResolveFailed { host: String, reason: String },
    /// The target (literal or any resolved IP) is in a blocked internal range.
    BlockedAddress { host: String },
}

/// Shared, tauri-free SSRF guard for user-supplied outbound URLs. Parses `raw`,
/// rejects non-http(s) schemes, and blocks targets that resolve to
/// loopback / link-local / private / ULA / CGNAT / unspecified / reserved
/// addresses. Hostnames are resolved via the non-blocking
/// `tokio::net::lookup_host` and rejected if ANY resolved IP is blocked. A
/// hostname listed in `CC_SWITCH_WEB_SSRF_ALLOW` bypasses these checks.
///
/// This is the single source of truth; both `validate_outbound_url`
/// (web request guard) and the usage-script guard delegate here to avoid
/// divergence. It is web-runtime-only by policy — the desktop runtime calls it
/// with `enforce = false` at the boundary so local dials stay unrestricted.
pub async fn guard_outbound_url(raw: &str) -> Result<(), OutboundUrlError> {
    let url = Url::parse(raw).map_err(|err| OutboundUrlError::InvalidUrl {
        raw: raw.to_string(),
        reason: err.to_string(),
    })?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(OutboundUrlError::UnsupportedScheme {
                scheme: other.to_string(),
            });
        }
    }

    let host = url.host().ok_or_else(|| OutboundUrlError::MissingHost {
        raw: raw.to_string(),
    })?;

    match host {
        Host::Ipv4(ip) => {
            if !ssrf_host_allowed(&ip.to_string()) && is_blocked_ipv4(&ip) {
                return Err(OutboundUrlError::BlockedAddress {
                    host: ip.to_string(),
                });
            }
        }
        Host::Ipv6(ip) => {
            if !ssrf_host_allowed(&ip.to_string()) && is_blocked_ip(IpAddr::V6(ip)) {
                return Err(OutboundUrlError::BlockedAddress {
                    host: ip.to_string(),
                });
            }
        }
        Host::Domain(domain) => {
            if ssrf_host_allowed(domain) {
                return Ok(());
            }
            let port = url.port_or_known_default().unwrap_or(0);
            let addrs = tokio::net::lookup_host((domain, port))
                .await
                .map_err(|err| OutboundUrlError::ResolveFailed {
                    host: domain.to_string(),
                    reason: err.to_string(),
                })?;
            for addr in addrs {
                if is_blocked_ip(addr.ip()) {
                    return Err(OutboundUrlError::BlockedAddress {
                        host: domain.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn blocks_unspecified_addresses() {
        // 0.0.0.0 and :: route to localhost on Linux — must be blocked.
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
        assert!(is_blocked_ipv4(&Ipv4Addr::new(0, 0, 0, 0)));
        // v4-mapped unspecified must also be caught via the unwrap path.
        assert!(is_blocked_ip(IpAddr::V6(
            "::ffff:0.0.0.0".parse::<Ipv6Addr>().unwrap()
        )));
    }

    #[test]
    fn blocks_cgnat_range() {
        // 100.64.0.0/10 (RFC 6598 / Tailscale).
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
        // Edges just outside the /10 must NOT be blocked.
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 63, 255, 255))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 0))));
    }

    #[test]
    fn blocks_classic_internal_ranges() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn allows_normal_public_addresses() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(140, 82, 121, 4))));
        assert!(!is_blocked_ip(IpAddr::V6(
            "2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap()
        )));
    }

    #[test]
    fn blocks_exotic_ipv4_ranges() {
        // 0.0.0.0/8 non-zero hosts (RFC 1122 "this network").
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(0, 1, 2, 3))));
        // Multicast 224.0.0.0/4.
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250))));
        // Reserved 240.0.0.0/4 + broadcast.
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
        // A normal public address right below the reserved block stays allowed.
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(223, 255, 255, 1))));
    }

    #[test]
    fn blocks_exotic_ipv6_ranges() {
        // IPv4-compatible `::a.b.c.d` (deprecated form to_ipv4_mapped misses).
        assert!(is_blocked_ip(IpAddr::V6(
            "::127.0.0.1".parse::<Ipv6Addr>().unwrap()
        )));
        assert!(is_blocked_ip(IpAddr::V6(
            "::10.0.0.1".parse::<Ipv6Addr>().unwrap()
        )));
        // 6to4 2002::/16.
        assert!(is_blocked_ip(IpAddr::V6(
            "2002:7f00:1::".parse::<Ipv6Addr>().unwrap()
        )));
        // NAT64 well-known prefix 64:ff9b::/96.
        assert!(is_blocked_ip(IpAddr::V6(
            "64:ff9b::7f00:1".parse::<Ipv6Addr>().unwrap()
        )));
        // Teredo 2001::/32.
        assert!(is_blocked_ip(IpAddr::V6(
            "2001:0:1234::1".parse::<Ipv6Addr>().unwrap()
        )));
        // IPv6 multicast ff00::/8.
        assert!(is_blocked_ip(IpAddr::V6(
            "ff02::1".parse::<Ipv6Addr>().unwrap()
        )));
        // Normal public v6 still allowed.
        assert!(!is_blocked_ip(IpAddr::V6(
            "2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap()
        )));
    }

    #[tokio::test]
    async fn guard_outbound_url_blocks_internal_and_allows_public() {
        assert!(guard_outbound_url("http://127.0.0.1/").await.is_err());
        assert!(guard_outbound_url("http://169.254.169.254/").await.is_err());
        assert!(guard_outbound_url("http://10.0.0.1/").await.is_err());
        assert!(guard_outbound_url("http://100.64.0.1/").await.is_err());
        assert!(guard_outbound_url("http://0.1.2.3/").await.is_err());
        assert!(guard_outbound_url("http://[::1]/").await.is_err());
        // Non-http(s) scheme rejected.
        assert!(guard_outbound_url("file:///etc/passwd").await.is_err());
        assert!(guard_outbound_url("ftp://1.1.1.1/").await.is_err());
        // Public IP literals allowed.
        assert!(guard_outbound_url("https://1.1.1.1/").await.is_ok());
        assert!(guard_outbound_url("https://8.8.8.8/").await.is_ok());
    }
}
