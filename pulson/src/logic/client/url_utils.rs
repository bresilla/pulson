/// Utility functions for building URLs for API requests
/// Supports both traditional host:port format and modern base URL format

/// Build API URL from either base_url or host/port combination
/// 
/// If base_url is provided, it takes precedence and should include the protocol.
/// Otherwise, constructs URL from host and port using http as default protocol.
pub fn build_api_url(
    base_url: Option<&str>,
    host: &str,
    port: u16,
    path: &str,
) -> String {
    if let Some(base) = base_url {
        // base_url takes precedence and should include protocol
        format!("{}{}", base.trim_end_matches('/'), path)
    } else {
        // Check if host already includes protocol
        if host.starts_with("http://") || host.starts_with("https://") {
            // Host already includes protocol, use as-is
            format!("{}{}", host.trim_end_matches('/'), path)
        } else {
            // No protocol specified, default to http
            format!("http://{}:{}{}", host, port, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_url_with_base_url() {
        let url = build_api_url(Some("https://sub.domain.com"), "127.0.0.1", 3030, "/api/devices");
        assert_eq!(url, "https://sub.domain.com/api/devices");
    }

    #[test]
    fn test_build_api_url_with_base_url_trailing_slash() {
        let url = build_api_url(Some("https://sub.domain.com/"), "127.0.0.1", 3030, "/api/devices");
        assert_eq!(url, "https://sub.domain.com/api/devices");
    }

    #[test]
    fn test_build_api_url_without_base_url() {
        let url = build_api_url(None, "127.0.0.1", 3030, "/api/devices");
        assert_eq!(url, "http://127.0.0.1:3030/api/devices");
    }

    #[test]
    fn test_build_api_url_host_with_protocol() {
        let url = build_api_url(None, "https://example.com", 3030, "/api/devices");
        assert_eq!(url, "https://example.com/api/devices");
    }

    #[test]
    fn test_build_api_url_host_with_protocol_and_port() {
        let url = build_api_url(None, "https://example.com:8443", 3030, "/api/devices");
        assert_eq!(url, "https://example.com:8443/api/devices");
    }
}
