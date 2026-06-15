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
//! Note: this module only classifies addresses; it never resolves DNS and never
//! performs IO, so it is safe to call from a sync redirect policy callback.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns true if the IPv4 address falls in a range the server must never dial
/// outbound: loopback (127.0.0.0/8), link-local (169.254.0.0/16), any private
/// range (10/8, 172.16/12, 192.168/16), the unspecified address (`0.0.0.0`,
/// which routes to localhost on Linux), or the CGNAT shared range
/// (100.64.0.0/10, RFC 6598 — also Tailscale's address space).
pub fn is_blocked_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_link_local()
        || ip.is_private()
        || ip.is_unspecified()
        || is_cgnat_ipv4(ip)
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
pub fn is_blocked_ipv6(ip: &Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
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
}
