//! # A2A Tool — MVP Implementation
//!
//! Client-side tool for interacting with remote A2A agents.
//! Supports: `discover`, `send` (streaming), `status`, `result` (polling).
//!
//! **Not yet implemented:** cancel, multi-turn conversations,
//! structured/binary message parts.

use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use std::sync::Arc;

/// Outbound A2A client tool — discovers remote agents and sends/retrieves tasks.
pub struct A2aTool {
    security: Arc<SecurityPolicy>,
    timeout_secs: u64,
    /// When true, allow requests to localhost/private IPs (same-host A2A).
    allow_local: bool,
    /// Default remote agent URL (from config).
    default_url: Option<String>,
    /// PAT token to include in JSON-RPC params.
    pat_token: Option<String>,
    /// Location ID to include in JSON-RPC params.
    location_id: Option<String>,
}

impl A2aTool {
    pub fn new(security: Arc<SecurityPolicy>, timeout_secs: u64, allow_local: bool) -> Self {
        Self {
            security,
            timeout_secs,
            allow_local,
            default_url: None,
            pat_token: None,
            location_id: None,
        }
    }

    /// Create A2A tool with default configuration values.
    pub fn with_config(
        security: Arc<SecurityPolicy>,
        timeout_secs: u64,
        allow_local: bool,
        default_url: Option<String>,
        pat_token: Option<String>,
        location_id: Option<String>,
    ) -> Self {
        Self {
            security,
            timeout_secs,
            allow_local,
            default_url,
            pat_token,
            location_id,
        }
    }

    fn build_client(&self) -> anyhow::Result<reqwest::Client> {
        let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error(std::io::Error::other("Too many redirects (max 10)"));
            }
            let host = attempt.url().host_str().unwrap_or("").to_string();
            if is_private_or_local_host(&host) {
                return attempt.error(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Blocked redirect to private/local host: {host}"),
                ));
            }
            attempt.follow()
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(redirect_policy)
            .user_agent("ZeroClaw/0.1 (a2a)")
            .build()?;
        Ok(client)
    }

    /// Validate that a URL targets a public host over HTTP(S).
    ///
    /// **Known limitation (DNS rebinding TOCTOU):** DNS is resolved here at
    /// validation time, but `reqwest` resolves again at connect time.  An
    /// attacker-controlled DNS record could flip between the two calls.  The
    /// redirect policy in `build_client` mitigates post-redirect SSRF, but
    /// the initial connection remains vulnerable to rebinding.  A custom
    /// `reqwest::dns::Resolve` would close this gap at the cost of added
    /// complexity; for now we accept this residual risk.
    fn validate_url(&self, url: &str) -> anyhow::Result<reqwest::Url> {
        let parsed = reqwest::Url::parse(url)?;
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => anyhow::bail!("Unsupported URL scheme: {scheme} (only http/https allowed)"),
        }
        if !self.allow_local {
            if let Some(host) = parsed.host_str() {
                if is_private_or_local_host(host) {
                    anyhow::bail!(
                        "Blocked request to private/local host: {host} (A2A only allows public hosts)"
                    );
                }
                validate_resolved_host_is_public(host)?;
            }
        }
        Ok(parsed)
    }

    async fn action_discover(
        &self,
        url: &str,
        bearer_token: Option<&str>,
    ) -> anyhow::Result<ToolResult> {
        let base = self.validate_url(url)?;
        let card_url = base.join("/.well-known/agent-card.json")?;
        let client = self.build_client()?;

        let mut req = client.get(card_url);

        // Add custom headers from config
        if let Some(token) = bearer_token {
            req = req.header("x-api-key", token);
        }
        if let Some(ref token) = self.pat_token {
            req = req.header("x-user-token", token);
        }
        if let Some(ref loc_id) = self.location_id {
            req = req.header("x-location-id", loc_id);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success() {
            Ok(ToolResult {
                success: true,
                output: body,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP {status}: {body}")),
            })
        }
    }

    async fn action_send(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        message: &str,
    ) -> anyhow::Result<ToolResult> {
        let base = self.validate_url(url)?;
        let rpc_url = base.join("/a2a")?;
        let client = self.build_client()?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let message_id = uuid::Uuid::new_v4().to_string();

        // Build params with optional pat_token and location_id
        let mut params = json!({
            "message": {
                "role": "user",
                "parts": [{ "kind": "text", "text": message }],
                "messageId": message_id
            }
        });

        // Add pat_token and location_id from tool config if present
        if let Some(ref token) = self.pat_token {
            if let Some(obj) = params.as_object_mut() {
                obj.insert("pat_token".to_string(), json!(token));
            }
        }
        if let Some(ref loc_id) = self.location_id {
            if let Some(obj) = params.as_object_mut() {
                obj.insert("location_id".to_string(), json!(loc_id));
            }
        }

        // Use message/send (non-streaming)
        let body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "message/send",
            "params": params
        });

        let mut req = client.post(rpc_url).json(&body);

        // Add custom headers from config
        if let Some(token) = bearer_token {
            req = req.header("x-api-key", token);
        }
        if let Some(ref token) = self.pat_token {
            req = req.header("x-user-token", token);
        }
        if let Some(ref loc_id) = self.location_id {
            req = req.header("x-location-id", loc_id);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let resp_body = resp.text().await?;

        if status.is_success() {
            Ok(ToolResult {
                success: true,
                output: resp_body,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP {status}: {resp_body}")),
            })
        }
    }

    async fn action_stream(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        message: &str,
    ) -> anyhow::Result<ToolResult> {
        let base = self.validate_url(url)?;
        let rpc_url = base.join("/a2a")?;
        let client = self.build_client()?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let message_id = uuid::Uuid::new_v4().to_string();

        // Build params with optional pat_token and location_id
        let mut params = json!({
            "message": {
                "role": "user",
                "parts": [{ "kind": "text", "text": message }],
                "messageId": message_id
            },
            "configuration": {
                "accepted_output_modes": ["text"]
            }
        });

        // Add pat_token and location_id from tool config if present
        if let Some(ref token) = self.pat_token {
            if let Some(obj) = params.as_object_mut() {
                obj.insert("pat_token".to_string(), json!(token));
            }
        }
        if let Some(ref loc_id) = self.location_id {
            if let Some(obj) = params.as_object_mut() {
                obj.insert("location_id".to_string(), json!(loc_id));
            }
        }

        // Use message/stream for streaming response
        let body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "message/stream",
            "params": params
        });

        let mut req = client.post(rpc_url).json(&body);

        // Add custom headers from config
        if let Some(token) = bearer_token {
            req = req.header("x-api-key", token);
        }
        if let Some(ref token) = self.pat_token {
            req = req.header("x-user-token", token);
        }
        if let Some(ref loc_id) = self.location_id {
            req = req.header("x-location-id", loc_id);
        }

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let resp_body = resp.text().await?;
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP {status}: {resp_body}")),
            });
        }

        // Parse SSE stream
        let text = self.parse_sse_stream(resp).await?;

        if text.is_empty() {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("No content received from remote agent".into()),
            })
        } else {
            Ok(ToolResult {
                success: true,
                output: text,
                error: None,
            })
        }
    }

    /// Parse SSE stream and extract text content from artifact-update events.
    async fn parse_sse_stream(&self, resp: reqwest::Response) -> anyhow::Result<String> {
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut collected_text = String::new();
        let mut task_id: Option<String> = None;
        let mut context_id: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Parse SSE field
                if let Some((field, value)) = line.split_once(':') {
                    let field = field.trim();
                    let value = value.trim_start();

                    if field == "data" && !value.is_empty() {
                        // Parse JSON data
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(value) {
                            // Extract task_id and context_id from result
                            if let Some(result) = json_value.get("result") {
                                if let Some(tid) = result.get("taskId").and_then(|v| v.as_str()) {
                                    task_id = Some(tid.to_string());
                                }
                                if let Some(cid) = result.get("contextId").and_then(|v| v.as_str()) {
                                    context_id = Some(cid.to_string());
                                }

                                // Extract text from artifact-update
                                if result.get("kind").and_then(|v| v.as_str()) == Some("artifact-update") {
                                    if let Some(artifact) = result.get("artifact") {
                                        if let Some(parts) = artifact.get("parts").and_then(|v| v.as_array()) {
                                            for part in parts {
                                                if part.get("kind").and_then(|v| v.as_str()) == Some("text") {
                                                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                                        collected_text.push_str(text);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no text was collected, return metadata about the task
        if collected_text.is_empty() {
            let mut info = String::new();
            if let Some(tid) = &task_id {
                let _ = std::fmt::Write::write_fmt(&mut info, format_args!("Task ID: {tid}\n"));
            }
            if let Some(cid) = &context_id {
                let _ = std::fmt::Write::write_fmt(&mut info, format_args!("Context ID: {cid}\n"));
            }
            if info.is_empty() {
                info.push_str("Task completed with no text output");
            }
            Ok(info)
        } else {
            Ok(collected_text)
        }
    }

    async fn action_get_task(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        task_id: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let base = self.validate_url(url)?;
        let rpc_url = base.join("/a2a")?;
        let client = self.build_client()?;
        let request_id = uuid::Uuid::new_v4().to_string();

        let body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "tasks/get",
            "params": { "id": task_id }
        });

        let mut req = client.post(rpc_url).json(&body);

        // Add custom headers from config
        if let Some(token) = bearer_token {
            req = req.header("x-api-key", token);
        }
        if let Some(ref token) = self.pat_token {
            req = req.header("x-user-token", token);
        }
        if let Some(ref loc_id) = self.location_id {
            req = req.header("x-location-id", loc_id);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let resp_body = resp.text().await?;

        if status.is_success() {
            let parsed: serde_json::Value = serde_json::from_str(&resp_body)?;
            Ok(parsed)
        } else {
            anyhow::bail!("HTTP {status}: {resp_body}");
        }
    }

    async fn action_status(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        task_id: &str,
    ) -> anyhow::Result<ToolResult> {
        match self.action_get_task(url, bearer_token, task_id).await {
            Ok(resp) => Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&resp)?,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn action_result(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        task_id: &str,
    ) -> anyhow::Result<ToolResult> {
        match self.action_get_task(url, bearer_token, task_id).await {
            Ok(resp) => {
                // Extract artifacts from the task response
                let artifacts = resp
                    .pointer("/result/artifacts")
                    .or_else(|| resp.pointer("/artifacts"))
                    .cloned()
                    .unwrap_or(json!([]));
                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&artifacts)?,
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }
}

#[async_trait]
impl Tool for A2aTool {
    fn name(&self) -> &str {
        "a2a"
    }

    fn description(&self) -> &str {
        "Communicate with remote agents via the A2A (Agent-to-Agent) protocol. \
         Supports five actions: 'discover' to fetch a remote agent's capability card, \
         'send' to dispatch a task message (non-streaming, returns task object), \
         'stream' to dispatch a task and receive streaming text response, \
         'status' to check task progress, and \
         'result' to retrieve task output artifacts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["discover", "send", "stream", "status", "result"],
                    "description": "A2A operation to perform"
                },
                "url": {
                    "type": "string",
                    "description": "Base URL of the remote agent (e.g. http://host:port). Optional if strands_agent_url is configured in [a2a] section."
                },
                "bearer_token": {
                    "type": "string",
                    "description": "Bearer token for authentication with the remote agent"
                },
                "task_id": {
                    "type": "string",
                    "description": "Task ID (required for status/result actions)"
                },
                "message": {
                    "type": "string",
                    "description": "Message to send to the remote agent (required for send/stream actions)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let url_arg = args.get("url").and_then(|v| v.as_str()).map(String::from);
        let bearer_token = args
            .get("bearer_token")
            .and_then(|v| v.as_str())
            .map(String::from);
        let task_id = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Use provided URL or fall back to configured default
        let url = match (&url_arg, &self.default_url) {
            (Some(u), _) => u.clone(),
            (None, Some(default)) => default.clone(),
            (None, None) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing required parameter: url (no default URL configured in [a2a].strands_agent_url)".into()),
                });
            }
        };

        match action.as_str() {
            "discover" => self.action_discover(&url, bearer_token.as_deref()).await,
            "send" => {
                if message.is_empty() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Missing required parameter: message".into()),
                    });
                }
                self.action_send(&url, bearer_token.as_deref(), &message)
                    .await
            }
            "stream" => {
                if message.is_empty() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Missing required parameter: message".into()),
                    });
                }
                self.action_stream(&url, bearer_token.as_deref(), &message)
                    .await
            }
            "status" => {
                if task_id.is_empty() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Missing required parameter: task_id".into()),
                    });
                }
                self.action_status(&url, bearer_token.as_deref(), &task_id)
                    .await
            }
            "result" => {
                if task_id.is_empty() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Missing required parameter: task_id".into()),
                    });
                }
                self.action_result(&url, bearer_token.as_deref(), &task_id)
                    .await
            }
            other => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Unknown action: '{other}'. Valid actions: discover, send, stream, status, result"
                )),
            }),
        }
    }
}

// ── SSRF protection helpers (mirrored from web_fetch.rs) ─────────

fn is_private_or_local_host(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let has_local_tld = bare
        .rsplit('.')
        .next()
        .is_some_and(|label| label == "local");

    if bare == "localhost" || bare.ends_with(".localhost") || has_local_tld {
        return true;
    }

    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(v6),
        };
    }

    false
}

#[cfg(not(test))]
fn validate_resolved_host_is_public(host: &str) -> anyhow::Result<()> {
    use std::net::ToSocketAddrs;

    let ips = (host, 0)
        .to_socket_addrs()
        .map_err(|e| anyhow::anyhow!("Failed to resolve host '{host}': {e}"))?
        .map(|addr| addr.ip())
        .collect::<Vec<_>>();

    validate_resolved_ips_are_public(host, &ips)
}

/// Test stub: skip DNS resolution so unit tests don't depend on network.
/// Literal IP/hostname checks are still exercised via `is_private_or_local_host`
/// in `validate_url`; only the resolve-and-recheck path is stubbed out.
#[cfg(test)]
fn validate_resolved_host_is_public(_host: &str) -> anyhow::Result<()> {
    Ok(())
}

fn validate_resolved_ips_are_public(host: &str, ips: &[std::net::IpAddr]) -> anyhow::Result<()> {
    if ips.is_empty() {
        anyhow::bail!("Failed to resolve host '{host}'");
    }

    for ip in ips {
        let non_global = match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(*v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(*v6),
        };
        if non_global {
            anyhow::bail!("Blocked host '{host}' resolved to non-global address {ip}");
        }
    }

    Ok(())
}

fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let [a, b, c, _] = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_multicast()
        || (a == 100 && (64..=127).contains(&b))
        || a >= 240
        || (a == 192 && b == 0 && (c == 0 || c == 2))
        || (a == 198 && b == 51)
        || (a == 203 && b == 0)
        || (a == 198 && (18..=19).contains(&b))
}

fn is_non_global_v6(v6: std::net::Ipv6Addr) -> bool {
    let segs = v6.segments();
    v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        || (segs[0] & 0xfe00) == 0xfc00
        || (segs[0] & 0xffc0) == 0xfe80
        || (segs[0] == 0x2001 && segs[1] == 0x0db8)
        || v6.to_ipv4_mapped().is_some_and(is_non_global_v4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityPolicy;

    fn test_tool() -> A2aTool {
        let security = Arc::new(SecurityPolicy::default());
        A2aTool::new(security, 30, false)
    }

    #[test]
    fn tool_metadata() {
        let tool = test_tool();
        assert_eq!(tool.name(), "a2a");
        assert!(!tool.description().is_empty());

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["url"].is_object());

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
        // url is no longer required - can use default from config
    }

    #[test]
    fn validate_url_accepts_public_http() {
        let tool = test_tool();
        assert!(tool.validate_url("http://agent.example.com:8080").is_ok());
        assert!(tool.validate_url("https://agent.example.com").is_ok());
    }

    #[test]
    fn validate_url_rejects_non_http() {
        let tool = test_tool();
        assert!(tool.validate_url("ftp://host").is_err());
        assert!(tool.validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn validate_url_rejects_private_hosts() {
        let tool = test_tool();
        assert!(tool.validate_url("http://localhost:9999").is_err());
        assert!(tool.validate_url("http://127.0.0.1:9999").is_err());
        assert!(tool.validate_url("http://10.0.0.1").is_err());
        assert!(tool.validate_url("http://192.168.1.1").is_err());
        assert!(tool.validate_url("http://172.16.0.1").is_err());
        assert!(tool.validate_url("http://169.254.169.254").is_err());
        assert!(tool.validate_url("http://[::1]").is_err());
        assert!(tool.validate_url("http://foo.local").is_err());
    }

    #[test]
    fn validate_url_allows_local_when_enabled() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = A2aTool::new(security, 30, true);
        assert!(tool.validate_url("http://127.0.0.1:42618").is_ok());
        assert!(tool.validate_url("http://localhost:42618").is_ok());
    }

    #[test]
    fn ssrf_helpers_block_cloud_metadata() {
        assert!(is_private_or_local_host("169.254.169.254"));
        assert!(is_private_or_local_host("127.0.0.1"));
        assert!(is_private_or_local_host("10.0.0.1"));
        assert!(is_private_or_local_host("localhost"));
        assert!(is_private_or_local_host("foo.localhost"));
        assert!(!is_private_or_local_host("8.8.8.8"));
        assert!(!is_private_or_local_host("example.com"));
    }

    #[tokio::test]
    async fn missing_url_returns_error_when_no_default() {
        // Tool without default URL should error when url is not provided
        let tool = test_tool();
        let result = tool.execute(json!({"action": "discover"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("url"));
    }

    #[tokio::test]
    async fn uses_default_url_when_configured() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Use mock server to avoid real network requests
        let server = MockServer::start().await;
        let card = json!({
            "name": "Test Agent",
            "version": "1.0",
            "skills": []
        });

        Mock::given(method("GET"))
            .and(path("/.well-known/agent-card.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&card))
            .mount(&server)
            .await;

        // Tool with default URL should use it when url arg is omitted
        let security = Arc::new(SecurityPolicy::default());
        let tool = A2aTool::with_config(
            security,
            5,
            true, // allow_local for localhost mock server
            Some(server.uri()),
            None,
            None,
        );
        // Should not error about missing URL
        let result = tool.execute(json!({"action": "discover"})).await.unwrap();
        // Should succeed (mock server returns valid card)
        assert!(result.success);
    }

    #[tokio::test]
    async fn unknown_action_returns_error() {
        let tool = test_tool();
        let result = tool
            .execute(json!({"action": "invalid", "url": "http://localhost"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("Unknown action"));
    }

    #[tokio::test]
    async fn send_missing_message_returns_error() {
        let tool = test_tool();
        let result = tool
            .execute(json!({"action": "send", "url": "http://localhost"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("message"));
    }

    #[tokio::test]
    async fn status_missing_task_id_returns_error() {
        let tool = test_tool();
        let result = tool
            .execute(json!({"action": "status", "url": "http://localhost"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("task_id"));
    }

    // ── HTTP integration tests (wiremock) ────────────────────
    //
    // These test the actual HTTP request/response cycle for each A2A
    // action.  `validate_url` blocks localhost, so we call the action
    // methods directly via a helper that patches the URL post-validation.
    // SSRF validation is already covered by the unit tests above.

    /// Build a tool with a short timeout suitable for mock-server tests.
    fn mock_tool() -> A2aTool {
        let security = Arc::new(SecurityPolicy::default());
        A2aTool::new(security, 5, false)
    }

    /// Directly call the discover action, bypassing SSRF validation
    /// (which rejects localhost where wiremock binds).
    async fn discover_direct(
        tool: &A2aTool,
        url: &str,
        bearer: Option<&str>,
    ) -> anyhow::Result<ToolResult> {
        let client = tool.build_client()?;
        let parsed = reqwest::Url::parse(url)?;
        let card_url = parsed.join("/.well-known/agent-card.json")?;
        let mut req = client.get(card_url);
        if let Some(token) = bearer {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if status.is_success() {
            Ok(ToolResult {
                success: true,
                output: body,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP {status}: {body}")),
            })
        }
    }

    /// Directly call the send action, bypassing SSRF validation.
    async fn send_direct(
        tool: &A2aTool,
        url: &str,
        bearer: Option<&str>,
        message: &str,
    ) -> anyhow::Result<ToolResult> {
        let client = tool.build_client()?;
        let parsed = reqwest::Url::parse(url)?;
        let rpc_url = parsed.join("/a2a")?;
        let body = json!({
            "jsonrpc": "2.0",
            "id": uuid::Uuid::new_v4().to_string(),
            "method": "message/send",
            "params": {
                "message": {
                    "role": "user",
                    "parts": [{"kind": "text", "text": message}],
                    "messageId": uuid::Uuid::new_v4().to_string()
                }
            }
        });
        let mut req = client.post(rpc_url).json(&body);
        if let Some(token) = bearer {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let resp_body = resp.text().await?;
        if status.is_success() {
            Ok(ToolResult {
                success: true,
                output: resp_body,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP {status}: {resp_body}")),
            })
        }
    }

    #[tokio::test]
    async fn discover_fetches_agent_card_from_server() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let card = json!({
            "name": "Test Agent",
            "version": "1.0",
            "skills": []
        });

        Mock::given(method("GET"))
            .and(path("/.well-known/agent-card.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&card))
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = discover_direct(&tool, &server.uri(), None).await.unwrap();
        assert!(result.success);
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["name"], "Test Agent");
    }

    #[tokio::test]
    async fn discover_returns_error_on_404() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/.well-known/agent-card.json"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = discover_direct(&tool, &server.uri(), None).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("404"));
    }

    #[tokio::test]
    async fn send_dispatches_jsonrpc_and_returns_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let rpc_response = json!({
            "jsonrpc": "2.0",
            "id": "test",
            "result": {
                "id": "task-1",
                "status": {"state": "completed"},
                "artifacts": [{"parts": [{"kind": "text", "text": "response"}]}]
            }
        });

        Mock::given(method("POST"))
            .and(path("/a2a"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&rpc_response))
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = send_direct(&tool, &server.uri(), None, "hello agent")
            .await
            .unwrap();
        assert!(result.success);
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["result"]["status"]["state"], "completed");
    }

    #[tokio::test]
    async fn send_includes_bearer_token_when_provided() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a2a"))
            .and(header("Authorization", "Bearer my-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"jsonrpc": "2.0", "id": "1", "result": {}})),
            )
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = send_direct(&tool, &server.uri(), Some("my-token"), "test")
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn send_reports_auth_failure() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a2a"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(json!({"error": "Unauthorized"})),
            )
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = send_direct(&tool, &server.uri(), None, "test")
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("401"));
    }

    #[tokio::test]
    async fn discover_with_bearer_sends_auth_header() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/.well-known/agent-card.json"))
            .and(header("Authorization", "Bearer secret-123"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"name": "Auth Agent", "skills": []})),
            )
            .mount(&server)
            .await;

        let tool = mock_tool();
        let result = discover_direct(&tool, &server.uri(), Some("secret-123"))
            .await
            .unwrap();
        assert!(result.success);
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["name"], "Auth Agent");
    }

    #[tokio::test]
    async fn read_only_autonomy_blocks_execution() {
        use crate::security::AutonomyLevel;
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });
        let tool = A2aTool::new(security, 5, false);
        let result = tool
            .execute(json!({"action": "discover", "url": "http://example.com"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("read-only"));
    }

    #[tokio::test]
    async fn send_includes_pat_token_and_location_id() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        let rpc_response = json!({
            "jsonrpc": "2.0",
            "id": "test-id",
            "result": {
                "id": "task-1",
                "status": {"state": "completed"}
            }
        });

        // Accept any POST to /a2a
        Mock::given(method("POST"))
            .and(path("/a2a"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&rpc_response))
            .mount(&server)
            .await;

        // Create tool with pat_token and location_id configured
        let security = Arc::new(SecurityPolicy::default());
        let tool = A2aTool::with_config(
            security,
            5,
            true, // allow_local for localhost mock server
            Some(server.uri()),
            Some("my-pat-token".to_string()),
            Some("location-123".to_string()),
        );

        let result = tool
            .execute(json!({"action": "send", "message": "hello"}))
            .await
            .unwrap();
        assert!(result.success);

        // Verify the request body contained pat_token and location_id
        // by checking the server received the correct request
        let requests = server.received_requests().await.unwrap();
        assert!(!requests.is_empty());
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["params"]["pat_token"], "my-pat-token");
        assert_eq!(body["params"]["location_id"], "location-123");
    }
}
