use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FaviconLookup {
    pub exact_authority: String,
    pub hostname: String,
    pub root_domain: Option<String>,
}

pub(super) fn extract_hosts(entry_url: &str) -> Option<FaviconLookup> {
    let parsed = Url::parse(entry_url).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let exact_authority = format_fetch_host(&host, parsed.port());
    let root = get_root_domain(&host);
    Some(FaviconLookup {
        exact_authority,
        hostname: host,
        root_domain: root,
    })
}

fn format_fetch_host(host: &str, port: Option<u16>) -> String {
    let host_for_url = if host.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    };

    if let Some(port) = port {
        format!("{host_for_url}:{port}")
    } else {
        host_for_url
    }
}

fn get_root_domain(host: &str) -> Option<String> {
    let registrable = get_public_registrable_domain(host)?;
    if registrable.eq_ignore_ascii_case(host) {
        return None;
    }
    Some(registrable)
}

pub(super) fn get_public_registrable_domain(host: &str) -> Option<String> {
    if host.parse::<std::net::Ipv4Addr>().is_ok() || host.parse::<std::net::Ipv6Addr>().is_ok() {
        return None;
    }

    // Only allow hosts whose suffix is in the Public Suffix List. PSL marks
    // suffixes derived via the implicit "*" wildcard rule (i.e. unknown
    // TLDs like .local, .internal, or any private/special-use name) as
    // unknown — those must not be sent to third-party favicon services.
    if !psl::suffix(host.as_bytes())?.is_known() {
        return None;
    }

    // For PSL-aware hosts, take the registrable domain (eTLD+1). When the
    // host already IS the registrable apex (example.co.uk) or sits on a
    // PSL platform suffix (user.github.io, app.netlify.app), domain_str
    // returns the host itself and the caller's equality check skips the
    // root-host fallback.
    psl::domain_str(host).map(str::to_ascii_lowercase)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn extract_hosts_normalizes_subdomain_and_root_domain() {
        let hosts = extract_hosts("https://APP.Example.COM/login").expect("extract hosts");

        assert_eq!(hosts.exact_authority, "app.example.com");
        assert_eq!(hosts.hostname, "app.example.com");
        assert_eq!(hosts.root_domain.as_deref(), Some("example.com"));
        assert_eq!(
            extract_hosts("https://login.example.co.uk")
                .expect("extract multi-label public suffix root")
                .root_domain
                .as_deref(),
            Some("example.co.uk")
        );

        let bare = extract_hosts("https://example.co.uk").expect("registrable apex");
        assert_eq!(bare.hostname, "example.co.uk");
        assert!(
            bare.root_domain.is_none(),
            "the registrable apex has no further root to fall back to"
        );

        let ip = extract_hosts("http://127.0.0.1:3000").expect("ip literal");
        assert_eq!(ip.exact_authority, "127.0.0.1:3000");
        assert_eq!(ip.hostname, "127.0.0.1");
        assert!(
            ip.root_domain.is_none(),
            "ip literals never produce a root host"
        );

        assert_eq!(extract_hosts("not a url"), None);
    }

    #[test]
    fn extract_hosts_preserves_explicit_ports_for_exact_fetches() {
        let hosts = extract_hosts("https://app.example.com:8443/login").expect("extract hosts");

        assert_eq!(hosts.exact_authority, "app.example.com:8443");
        assert_eq!(hosts.hostname, "app.example.com");
        assert_eq!(hosts.root_domain.as_deref(), Some("example.com"));
    }

    #[test]
    fn extract_hosts_rejects_non_http_schemes() {
        // Non-web entries (ssh, db, ldap, etc.) must not trigger favicon
        // network activity. extract_hosts returns None so fetch_entry_favicon
        // short-circuits with NotFound before any DNS or HTTP traffic.
        for url in [
            "ssh://bastion.corp.example/",
            "postgres://db.internal:5432/prod",
            "ldap://corp-ad.internal/",
            "file:///etc/hosts",
            "ftp://files.example.com/",
            "obsidian://open?vault=Notes",
        ] {
            assert_eq!(extract_hosts(url), None, "expected no fetch for {url}");
        }
    }

    #[test]
    fn extract_hosts_skips_root_fallback_for_psl_hosting_platforms() {
        // PSL entries like github.io / netlify.app / vercel.app are public
        // suffixes themselves, so user.github.io is already the registrable
        // apex. The root-host fallback must be skipped so we never fetch
        // the provider's marketing-page favicon by accident.
        for host in [
            "user.github.io",
            "team.netlify.app",
            "demo.vercel.app",
            "tenant.azurewebsites.net",
        ] {
            let url = format!("https://{host}/");
            let parsed = extract_hosts(&url).expect("parse psl host");
            assert_eq!(parsed.hostname, host);
            assert!(
                parsed.root_domain.is_none(),
                "{host} should not produce a root-host fallback"
            );
        }
    }
}
