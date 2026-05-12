use super::lookup::{get_public_registrable_domain, FaviconLookup};

const GOOGLE_FAVICON_URL: &str = "https://www.google.com/s2/favicons";
const ICON_HORSE_URL: &str = "https://icon.horse/icon";

#[derive(Debug, Clone)]
pub(super) struct FaviconCandidate {
    pub fetch_url: String,
    pub cooldown_domain: String,
}

pub(super) fn build_favicon_candidates(
    hosts: &FaviconLookup,
    allow_third_party_fallbacks: bool,
) -> Vec<FaviconCandidate> {
    let mut direct_hosts = vec![(hosts.exact_authority.as_str(), hosts.hostname.as_str())];
    if let Some(root) = hosts.root_domain.as_deref() {
        if root != hosts.hostname {
            direct_hosts.push((root, root));
        }
    }

    let mut candidates = Vec::new();
    for (fetch_host, cooldown_domain) in &direct_hosts {
        candidates.push(FaviconCandidate {
            fetch_url: format!("https://{fetch_host}/favicon.ico"),
            cooldown_domain: (*cooldown_domain).to_string(),
        });
    }

    if allow_third_party_fallbacks {
        let mut third_party_domains = Vec::new();
        if let Some(domain) = get_public_registrable_domain(&hosts.hostname) {
            third_party_domains.push(domain);
        }
        if let Some(root) = hosts.root_domain.as_deref() {
            if let Some(domain) = get_public_registrable_domain(root) {
                if !third_party_domains.contains(&domain) {
                    third_party_domains.push(domain);
                }
            }
        }

        for domain in &third_party_domains {
            candidates.push(FaviconCandidate {
                fetch_url: format!("{GOOGLE_FAVICON_URL}?domain={domain}&sz=64"),
                cooldown_domain: domain.clone(),
            });
            candidates.push(FaviconCandidate {
                fetch_url: format!("{ICON_HORSE_URL}/{domain}"),
                cooldown_domain: domain.clone(),
            });
        }
    }

    candidates
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn build_candidates_tries_hostname_before_root_domain() {
        let hosts = FaviconLookup {
            exact_authority: "app.example.com".to_string(),
            hostname: "app.example.com".to_string(),
            root_domain: Some("example.com".to_string()),
        };
        let candidates = build_favicon_candidates(&hosts, false);
        let urls: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.fetch_url.as_str())
            .collect();

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://app.example.com/favicon.ico");
        assert_eq!(urls[1], "https://example.com/favicon.ico");
    }

    #[test]
    fn build_candidates_adds_opt_in_third_party_sources_for_public_domains() {
        let hosts = FaviconLookup {
            exact_authority: "app.example.com".to_string(),
            hostname: "app.example.com".to_string(),
            root_domain: Some("example.com".to_string()),
        };
        let candidates = build_favicon_candidates(&hosts, true);

        assert!(candidates.iter().any(|candidate| {
            candidate.fetch_url == "https://www.google.com/s2/favicons?domain=example.com&sz=64"
        }));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.fetch_url == "https://icon.horse/icon/example.com"));
        assert!(!candidates.iter().any(|candidate| {
            candidate.fetch_url.contains("domain=app.example.com")
                || candidate.fetch_url.ends_with("/app.example.com")
        }));
    }

    #[test]
    fn build_candidates_omits_third_party_sources_for_private_hosts() {
        let hosts = FaviconLookup {
            exact_authority: "nas.local:5001".to_string(),
            hostname: "nas.local".to_string(),
            root_domain: None,
        };
        let candidates = build_favicon_candidates(&hosts, true);
        let urls: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.fetch_url.as_str())
            .collect();

        assert_eq!(urls, vec!["https://nas.local:5001/favicon.ico"]);
    }

    #[test]
    fn build_candidates_with_explicit_port_keeps_port_off_root_fallbacks() {
        let hosts = FaviconLookup {
            exact_authority: "app.example.com:8443".to_string(),
            hostname: "app.example.com".to_string(),
            root_domain: Some("example.com".to_string()),
        };
        let candidates = build_favicon_candidates(&hosts, true);
        let urls: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.fetch_url.as_str())
            .collect();

        assert_eq!(urls[0], "https://app.example.com:8443/favicon.ico");
        assert_eq!(urls[1], "https://example.com/favicon.ico");
        assert!(urls.contains(&"https://www.google.com/s2/favicons?domain=example.com&sz=64"));
        assert!(!urls
            .iter()
            .skip(2)
            .any(|url| url.contains("app.example.com:8443")));
    }
}
