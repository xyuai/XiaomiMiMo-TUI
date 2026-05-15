//! Direct-fetch HTTP tool. Complements `web_search` for cases where the user
//! already knows the URL — a known repo, a blog post, a spec page — and
//! search is overkill or actively unhelpful.
//!
//! Returns a structured `{url, status, content_type, content, truncated}`
//! payload. HTML responses are stripped to readable text by default
//! (`format = "markdown"`); pass `format = "raw"` to keep the bytes intact
//! when the model wants to do its own parsing.

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, optional_u64,
};
use crate::network_policy::Decision;
use async_trait::async_trait;
use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;

const DEFAULT_MAX_BYTES: u64 = 1_000_000;
const HARD_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const HARD_MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_REDIRECTS: usize = 5;
const USER_AGENT: &str =
    "Mozilla/5.0 (compatible; xiaomimimo-tui/0.5; +https://github.com/xyuai/XiaomiMiMo-TUI)";

static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
static STYLE_RE: OnceLock<Regex> = OnceLock::new();
static TAG_RE: OnceLock<Regex> = OnceLock::new();
static WHITESPACE_RE: OnceLock<Regex> = OnceLock::new();

fn script_re() -> &'static Regex {
    SCRIPT_RE.get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("script re"))
}
fn style_re() -> &'static Regex {
    STYLE_RE.get_or_init(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("style re"))
}
fn tag_re() -> &'static Regex {
    TAG_RE.get_or_init(|| Regex::new(r"<[^>]+>").expect("tag re"))
}
fn whitespace_re() -> &'static Regex {
    WHITESPACE_RE.get_or_init(|| Regex::new(r"\s+").expect("ws re"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Markdown,
    Raw,
}

impl Format {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value
            .unwrap_or("markdown")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "text" | "txt" | "plain" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "raw" | "html" | "bytes" => Ok(Self::Raw),
            other => Err(ToolError::invalid_input(format!(
                "unknown format `{other}` (allowed: text, markdown, raw)"
            ))),
        }
    }
}

#[derive(Debug, Serialize)]
struct FetchResponse {
    url: String,
    status: u16,
    content_type: String,
    content: String,
    truncated: bool,
}

pub struct FetchUrlTool;

#[async_trait]
impl ToolSpec for FetchUrlTool {
    fn name(&self) -> &'static str {
        "fetch_url"
    }

    fn description(&self) -> &'static str {
        "Fetch a known URL directly (HTTP GET) and return its content. Use this when the user gives a URL or you already know the canonical link — it's faster and more reliable than web_search for known pages."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Absolute HTTP/HTTPS URL to fetch."
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "raw"],
                    "description": "Post-processing for the response body. `markdown` (default) and `text` strip HTML tags to readable text; `raw` returns the body bytes as-is."
                },
                "max_bytes": {
                    "type": "integer",
                    "description": "Truncate response body after this many bytes (default 1,000,000; hard max 10,485,760)."
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Request timeout in milliseconds (default 15,000; max 60,000)."
                }
            },
            "required": ["url"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::invalid_input("`url` is required"))?
            .trim()
            .to_string();

        if url.is_empty() {
            return Err(ToolError::invalid_input("`url` cannot be empty"));
        }
        let scheme_ok = url.starts_with("http://") || url.starts_with("https://");
        if !scheme_ok {
            return Err(ToolError::invalid_input(
                "only http:// and https:// URLs are supported",
            ));
        }

        let format = Format::parse(input.get("format").and_then(Value::as_str))?;
        let max_bytes = optional_u64(&input, "max_bytes", DEFAULT_MAX_BYTES).min(HARD_MAX_BYTES);
        let timeout_ms =
            optional_u64(&input, "timeout_ms", DEFAULT_TIMEOUT_MS).min(HARD_MAX_TIMEOUT_MS);
        let mut current_url = reqwest::Url::parse(&url)
            .map_err(|e| ToolError::invalid_input(format!("invalid URL: {e}")))?;
        let mut redirects_followed = 0usize;

        let resp = loop {
            let dns_pinning = validate_fetch_target(&current_url, context).await?;
            let mut client_builder = reqwest::Client::builder()
                .timeout(Duration::from_millis(timeout_ms))
                .user_agent(USER_AGENT)
                .redirect(reqwest::redirect::Policy::none());

            if let Some((hostname, validated_ip)) = dns_pinning {
                client_builder =
                    client_builder.resolve(&hostname, SocketAddr::new(validated_ip, 0));
            }

            let client = client_builder.build().map_err(|e| {
                ToolError::execution_failed(format!("failed to build HTTP client: {e}"))
            })?;

            let resp = client
                .get(current_url.clone())
                .header("Accept", "text/html,text/plain,application/json,*/*;q=0.5")
                .header("Accept-Language", "en-US,en;q=0.5")
                .send()
                .await
                .map_err(|e| ToolError::execution_failed(format!("request failed: {e}")))?;

            if !resp.status().is_redirection() || redirects_followed >= MAX_REDIRECTS {
                break resp;
            }

            let Some(location) = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
            else {
                break resp;
            };

            current_url = resp.url().join(location).map_err(|e| {
                ToolError::execution_failed(format!("invalid redirect location: {e}"))
            })?;
            redirects_followed += 1;
        };

        let final_url = resp.url().to_string();
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ToolError::execution_failed(format!("failed to read body: {e}")))?;
        let total_bytes = bytes.len() as u64;
        let truncated = total_bytes > max_bytes;
        let usable = if truncated {
            &bytes[..max_bytes as usize]
        } else {
            &bytes[..]
        };

        let body_text = String::from_utf8_lossy(usable).to_string();
        let processed = match format {
            Format::Raw => body_text,
            Format::Text | Format::Markdown => {
                if content_type.contains("text/html") || body_text.contains("<html") {
                    html_to_text(&body_text)
                } else {
                    body_text
                }
            }
        };

        let response = FetchResponse {
            url: final_url,
            status: status.as_u16(),
            content_type,
            content: processed,
            truncated,
        };

        if !status.is_success() {
            // Don't `Err` on 4xx/5xx — the caller often wants to see the body
            // (e.g. a JSON error envelope). Mark the result as a failure so the
            // engine renders it as such.
            return Ok(ToolResult {
                content: serde_json::to_string_pretty(&response).map_err(|e| {
                    ToolError::execution_failed(format!("failed to serialize response: {e}"))
                })?,
                success: false,
                metadata: None,
            });
        }

        ToolResult::json(&response)
            .map_err(|e| ToolError::execution_failed(format!("failed to serialize response: {e}")))
    }
}

/// Return true for addresses that must not be reachable through an
/// LLM-initiated fetch: localhost, private/internal networks, link-local,
/// multicast, unspecified, cloud metadata, CGNAT, benchmarking, and reserved
/// ranges. IPv4-mapped IPv6 is checked as IPv4 to avoid bypasses.
fn is_restricted_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || matches!(v4.octets(), [100, 64..=127, ..])
                || *v4 == Ipv4Addr::new(169, 254, 169, 254)
                || matches!(v4.octets(), [198, 18..=19, ..])
                || v4.octets()[0] >= 240
        }
        IpAddr::V6(v6) => {
            if v6.is_unspecified()
                || matches!(v6.octets(), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, ..])
            {
                return true;
            }
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_restricted_ip(&IpAddr::V4(v4));
            }
            v6.is_loopback()
                || v6.is_multicast()
                || matches!(v6.segments(), [0xfc00..=0xfdff, ..])
                || matches!(v6.segments(), [0xfe80..=0xfebf, ..])
        }
    }
}

async fn validate_fetch_target(
    url: &reqwest::Url,
    context: &ToolContext,
) -> Result<Option<(String, IpAddr)>, ToolError> {
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(ToolError::invalid_input(
            "only http:// and https:// URLs are supported",
        ));
    }

    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| ToolError::invalid_input("URL must include a host"))?;

    validate_network_policy(&host, context)?;

    if matches!(host.as_str(), "localhost" | "localhost.localdomain") {
        return Err(ToolError::permission_denied(
            "requests to localhost are not allowed",
        ));
    }

    // Handle both normal host strings and bracketed IPv6 literals through the
    // same restricted-IP classifier.
    let ip_candidate = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host.as_str());
    if let Ok(ip) = ip_candidate.parse::<IpAddr>() {
        if is_restricted_ip(&ip) {
            return Err(ToolError::permission_denied(format!(
                "IP {ip} is a restricted address (private/loopback/link-local)"
            )));
        }
        return Ok(None);
    }

    let mut first_valid = None;
    if let Ok(addrs) = tokio::net::lookup_host((host.as_str(), 0u16)).await {
        for addr in addrs {
            let ip = addr.ip();
            validate_dns_resolved_ip(&host, &ip, context.network_policy.as_ref())?;
            first_valid.get_or_insert(ip);
        }
    }

    // If DNS resolution fails, let the HTTP request fail naturally so callers
    // still get the provider/network error text.
    Ok(first_valid.map(|ip| (host, ip)))
}

fn validate_network_policy(host: &str, context: &ToolContext) -> Result<(), ToolError> {
    let Some(decider) = context.network_policy.as_ref() else {
        return Ok(());
    };
    match decider.evaluate(host, "fetch_url") {
        Decision::Allow => Ok(()),
        Decision::Deny => Err(ToolError::permission_denied(format!(
            "network call to '{host}' blocked by network policy"
        ))),
        Decision::Prompt => Err(ToolError::permission_denied(format!(
            "network call to '{host}' requires approval; \
             re-run after `/network allow {host}` or set network.default = \"allow\" in config"
        ))),
    }
}

fn validate_dns_resolved_ip(
    host: &str,
    ip: &IpAddr,
    decider: Option<&crate::network_policy::NetworkPolicyDecider>,
) -> Result<(), ToolError> {
    if !is_restricted_ip(ip) {
        return Ok(());
    }

    if let Some(decider) = decider
        && decider.trusts_proxy_fakeip_host(host)
    {
        decider.record_trusted_proxy_fakeip_allow(host, "fetch_url");
        return Ok(());
    }

    Err(ToolError::permission_denied(format!(
        "resolved IP {ip} is a restricted address (private/loopback/link-local)"
    )))
}

/// Strip `<script>` / `<style>` blocks, drop remaining tags, and collapse
/// whitespace. Good enough for "let the model read this page" — not a full
/// HTML-to-Markdown converter.
fn html_to_text(html: &str) -> String {
    let no_script = script_re().replace_all(html, "");
    let no_style = style_re().replace_all(&no_script, "");
    let no_tags = tag_re().replace_all(&no_style, " ");
    let decoded = decode_entities(&no_tags);
    whitespace_re()
        .replace_all(&decoded, " ")
        .trim()
        .to_string()
}

/// Decode the handful of HTML entities we expect to hit in stripped text.
/// Pulling in `html-escape` for the long tail isn't worth the dep weight.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::spec::ToolContext;
    use std::path::PathBuf;

    fn ctx() -> ToolContext {
        ToolContext::new(PathBuf::from("."))
    }

    #[test]
    fn html_to_text_strips_scripts_styles_and_tags() {
        let html = r#"
            <html>
              <head>
                <style>body { color: red; }</style>
                <script>alert("nope");</script>
              </head>
              <body>
                <h1>Hello &amp; welcome</h1>
                <p>This is <b>important</b>.</p>
              </body>
            </html>
        "#;
        let text = html_to_text(html);
        assert!(text.contains("Hello & welcome"));
        assert!(text.contains("This is important"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color: red"));
    }

    #[test]
    fn format_parse_accepts_aliases_and_rejects_unknown() {
        assert_eq!(Format::parse(Some("markdown")).unwrap(), Format::Markdown);
        assert_eq!(Format::parse(Some("MD")).unwrap(), Format::Markdown);
        assert_eq!(Format::parse(Some("text")).unwrap(), Format::Text);
        assert_eq!(Format::parse(Some("raw")).unwrap(), Format::Raw);
        assert_eq!(Format::parse(None).unwrap(), Format::Markdown);
        assert!(Format::parse(Some("yaml")).is_err());
    }

    #[tokio::test]
    async fn rejects_non_http_schemes() {
        let tool = FetchUrlTool;
        let res = tool
            .execute(json!({"url": "file:///etc/passwd"}), &ctx())
            .await;
        let err = res.unwrap_err();
        assert!(format!("{err:?}").contains("http"));
    }

    #[tokio::test]
    async fn rejects_empty_url() {
        let tool = FetchUrlTool;
        let res = tool.execute(json!({"url": "   "}), &ctx()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_missing_url() {
        let tool = FetchUrlTool;
        let res = tool.execute(json!({}), &ctx()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn network_policy_denies_blocked_host() {
        use crate::network_policy::{Decision, NetworkPolicy, NetworkPolicyDecider};
        let policy = NetworkPolicy {
            default: Decision::Deny.into(),
            allow: vec!["token-plan-cn.xiaomimimo.com".to_string()],
            deny: vec![],
            proxy: vec![],
            audit: false,
        };
        let decider = NetworkPolicyDecider::new(policy, None);
        let ctx = ToolContext::new(PathBuf::from(".")).with_network_policy(decider);
        let tool = FetchUrlTool;
        let res = tool
            .execute(json!({"url": "https://example.com/foo"}), &ctx)
            .await;
        let err = res.expect_err("blocked host should fail");
        assert!(format!("{err}").contains("blocked"));
    }

    #[test]
    fn restricted_ip_classifier_blocks_local_internal_and_metadata() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "::",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(
                is_restricted_ip(&ip.parse().unwrap()),
                "{ip} should be blocked"
            );
        }
        for ip in ["8.8.8.8", "1.1.1.1", "2606:4700::1"] {
            assert!(
                !is_restricted_ip(&ip.parse().unwrap()),
                "{ip} should be allowed"
            );
        }
    }

    #[tokio::test]
    async fn rejects_localhost_hostname() {
        let tool = FetchUrlTool;
        let res = tool
            .execute(json!({"url": "http://localhost:12345"}), &ctx())
            .await;
        let err = res.expect_err("localhost should fail before request");
        assert!(format!("{err}").contains("localhost"));
    }

    #[tokio::test]
    async fn redirected_private_ip_literal_is_rejected_by_target_validator() {
        let url = reqwest::Url::parse("http://169.254.169.254/latest/meta-data").unwrap();
        let err = validate_fetch_target(&url, &ctx())
            .await
            .expect_err("metadata IP must be rejected");
        assert!(format!("{err}").contains("restricted address"));
    }

    #[tokio::test]
    async fn rejects_bracketed_ipv6_literal_loopback() {
        let url = reqwest::Url::parse("http://[::1]/").unwrap();
        let err = validate_fetch_target(&url, &ctx())
            .await
            .expect_err("[::1] must be rejected");
        assert!(format!("{err}").contains("restricted address"));
    }

    #[test]
    fn proxy_opt_in_allows_restricted_dns_for_matching_host_only() {
        use crate::network_policy::{Decision, NetworkPolicy, NetworkPolicyDecider};

        let policy = NetworkPolicy {
            default: Decision::Allow.into(),
            allow: vec![],
            deny: vec![],
            proxy: vec!["github.com".to_string()],
            audit: false,
        };
        let decider = NetworkPolicyDecider::new(policy, None);
        let ip = "198.18.0.1".parse().unwrap();

        validate_dns_resolved_ip("github.com", &ip, Some(&decider))
            .expect("matching proxy fake-IP host should be allowed");
        let err = validate_dns_resolved_ip("example.com", &ip, Some(&decider))
            .expect_err("unlisted host should stay blocked");
        assert!(format!("{err}").contains("restricted address"));
    }
}
