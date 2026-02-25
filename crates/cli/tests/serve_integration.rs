//! Integration tests for the `tenor serve` HTTP API.
//!
//! Each test starts the server as a child process on a unique port,
//! makes HTTP requests, and verifies the responses.

use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

/// Atomic port counter to avoid port conflicts between parallel tests.
/// Base port is derived from process ID so parallel `cargo test --workspace` runs
/// (which spawn separate test binaries) don't collide on the same port range.
static NEXT_PORT: AtomicU16 = AtomicU16::new(0);
static PORT_INIT: std::sync::Once = std::sync::Once::new();

fn next_port() -> u16 {
    PORT_INIT.call_once(|| {
        let base = 20000 + (std::process::id() as u16 % 20000);
        NEXT_PORT.store(base, Ordering::SeqCst);
    });
    NEXT_PORT.fetch_add(1, Ordering::SeqCst)
}

/// Helper: start the tenor serve process on the given port.
fn start_server(port: u16, contracts: &[&str]) -> Child {
    // The workspace root is two levels up from crates/cli
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tenor"));
    cmd.current_dir(workspace_root);
    cmd.arg("serve").arg("--port").arg(port.to_string());
    for c in contracts {
        cmd.arg(c);
    }
    // Redirect stdout/stderr to avoid blocking
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().expect("failed to start tenor serve");
    // Wait for server to be ready by polling the port
    for _ in 0..50 {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return child;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    child
}

/// Helper: make a simple HTTP GET request and return (status, body).
fn http_get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        path, port
    );
    std::io::Write::write_all(&mut stream, request.as_bytes()).expect("failed to write");

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    parse_http_response(&response)
}

/// Helper: make a simple HTTP POST request and return (status, body).
fn http_post(port: u16, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: localhost:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, port, body.len(), body
    );
    std::io::Write::write_all(&mut stream, request.as_bytes()).expect("failed to write");

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    parse_http_response(&response)
}

/// Helper: make an HTTP GET request with custom headers and return (status, response_headers, body).
fn http_get_with_headers(
    port: u16,
    path: &str,
    extra_headers: &[(&str, &str)],
) -> (u16, String, String) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    let mut header_lines = String::new();
    for (name, value) in extra_headers {
        header_lines.push_str(&format!("{}: {}\r\n", name, value));
    }

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost:{}\r\n{}Connection: close\r\n\r\n",
        path, port, header_lines
    );
    std::io::Write::write_all(&mut stream, request.as_bytes()).expect("failed to write");

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    parse_http_response_full(&response)
}

/// Extract a header value from raw headers string.
fn extract_header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    let name_lower = name.to_lowercase();
    for line in headers.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().to_lowercase() == name_lower {
                return Some(value.trim());
            }
        }
    }
    None
}

/// Parse an HTTP response into (status_code, body).
fn parse_http_response(response: &str) -> (u16, String) {
    let (status, _, body) = parse_http_response_full(response);
    (status, body)
}

/// Parse an HTTP response into (status_code, headers_string, body).
fn parse_http_response_full(response: &str) -> (u16, String, String) {
    let parts: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
    let headers = parts.first().unwrap_or(&"").to_string();
    let body = parts.get(1).unwrap_or(&"").to_string();

    let status_line = headers.lines().next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Handle chunked transfer encoding
    let body = if headers.contains("Transfer-Encoding: chunked") {
        decode_chunked(&body)
    } else {
        body
    };

    (status, headers, body)
}

/// Decode chunked transfer encoding.
fn decode_chunked(data: &str) -> String {
    let mut result = String::new();
    let mut remaining = data;

    while let Some(line_end) = remaining.find("\r\n") {
        let size_str = &remaining[..line_end];
        let size = match usize::from_str_radix(size_str.trim(), 16) {
            Ok(s) => s,
            Err(_) => break,
        };
        if size == 0 {
            break;
        }
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + size;
        if chunk_end > remaining.len() {
            // Partial chunk, take what we have
            result.push_str(&remaining[chunk_start..]);
            break;
        }
        result.push_str(&remaining[chunk_start..chunk_end]);
        // Skip past chunk data + \r\n
        remaining = if chunk_end + 2 <= remaining.len() {
            &remaining[chunk_end + 2..]
        } else {
            ""
        };
    }

    result
}

#[test]
fn health_returns_200_with_version() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_get(port, "/health");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(json["status"], "ok");
    assert!(
        json.get("tenor_version").is_some(),
        "tenor_version field must be present"
    );
}

#[test]
fn contracts_list_empty_when_no_preloads() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_get(port, "/contracts");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let contracts = json["contracts"].as_array().expect("contracts array");
    assert_eq!(contracts.len(), 0);
}

#[test]
fn contracts_list_with_preloaded() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let (status, body) = http_get(port, "/contracts");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let contracts = json["contracts"].as_array().expect("contracts array");
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0]["id"], "saas_subscription");
}

#[test]
fn operations_for_preloaded_contract() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let (status, body) = http_get(port, "/contracts/saas_subscription/operations");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let ops = json["operations"].as_array().expect("operations array");
    assert!(!ops.is_empty(), "should have operations");
    // Verify structure
    let first = &ops[0];
    assert!(first.get("id").is_some());
    assert!(first.get("allowed_personas").is_some());
    assert!(first.get("effects").is_some());
}

#[test]
fn operations_for_unknown_contract_returns_404() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_get(port, "/contracts/nonexistent/operations");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 404);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(json.get("error").is_some());
}

#[test]
fn elaborate_valid_source() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let source =
        "{\"source\": \"fact is_active {\\n  type: Bool\\n  source: \\\"system.active\\\"\\n}\"}";
    let (status, body) = http_post(port, "/elaborate", source);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "elaborate should succeed, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(json.get("constructs").is_some(), "should have constructs");
}

#[test]
fn elaborate_invalid_source_returns_400() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_post(
        port,
        "/elaborate",
        r#"{"source": "fact broken { type: ??? }"}"#,
    );
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 400);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(json.get("error").is_some());
    assert!(json.get("details").is_some());
}

#[test]
fn evaluate_with_preloaded_contract() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let facts = r#"{"bundle_id": "saas_subscription", "facts": {"current_seat_count": 15, "subscription_plan": "professional", "plan_features": {"max_seats": 50, "api_access": true, "sso_enabled": true, "custom_branding": false}, "payment_ok": true, "account_age_days": 14}}"#;
    let (status, body) = http_post(port, "/evaluate", facts);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "evaluate should succeed, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let verdicts = json["verdicts"].as_array().expect("verdicts array");
    assert!(!verdicts.is_empty(), "should produce verdicts");
}

#[test]
fn evaluate_unknown_bundle_returns_404() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_post(port, "/evaluate", r#"{"bundle_id": "nope", "facts": {}}"#);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 404);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(json.get("error").is_some());
}

#[test]
fn explain_preloaded_contract() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let (status, body) = http_post(port, "/explain", r#"{"bundle_id": "saas_subscription"}"#);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "explain should succeed, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(json.get("summary").is_some(), "should have summary field");
}

#[test]
fn not_found_returns_404() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let (status, body) = http_get(port, "/nonexistent");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 404);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(json["error"], "not found");
}

#[test]
fn elaborate_oversized_source_returns_400() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    // Create a source string that exceeds 1MB
    let big_source = "x".repeat(1024 * 1024 + 1);
    let body = serde_json::json!({"source": big_source}).to_string();
    let (status, resp_body) = http_post(port, "/elaborate", &body);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 400, "oversized source should be rejected");
    let json: serde_json::Value = serde_json::from_str(&resp_body).expect("valid JSON");
    assert_eq!(json["error"], "source content exceeds maximum size");
}

#[test]
fn elaborate_path_traversal_filename_sanitized() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    // A filename with path traversal should be sanitized, and elaboration
    // should still proceed with the default filename (source.tenor).
    let body = r#"{"source": "fact ok {\n  type: Bool\n  source: \"x\"\n}", "filename": "../../../etc/passwd"}"#;
    let (status, resp_body) = http_post(port, "/elaborate", body);
    child.kill().ok();
    child.wait().ok();

    // The request should succeed (filename is sanitized to source.tenor)
    // or fail with an elaboration error -- not a server error or path traversal
    assert!(
        status == 200 || status == 400,
        "should not be server error, got status {} body: {}",
        status,
        resp_body
    );
    // If it's a 400, it should be an elaboration error, not a path error
    if status == 400 {
        let json: serde_json::Value = serde_json::from_str(&resp_body).expect("valid JSON");
        let error = json["error"].as_str().unwrap_or("");
        assert!(
            !error.contains("passwd"),
            "error should not reference the traversal path"
        );
    }
}

#[test]
fn elaborate_import_escape_returns_400() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let body = r#"{"source": "import \"../../../etc/passwd\"\nfact ok {\n  type: Bool\n  source: \"x\"\n}"}"#;
    let (status, resp_body) = http_post(port, "/elaborate", body);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(
        status, 400,
        "import escape should be rejected, body: {}",
        resp_body
    );
    let json: serde_json::Value = serde_json::from_str(&resp_body).expect("valid JSON");
    assert_eq!(json["error"], "import paths must not escape sandbox");
}

#[test]
fn elaborate_null_byte_returns_400() {
    let port = next_port();
    let mut child = start_server(port, &[]);

    let body = r#"{"source": "fact ok {\n  type: Bool\n  source: \"x\"\n}\u0000"}"#;
    let (status, resp_body) = http_post(port, "/elaborate", body);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(
        status, 400,
        "null byte should be rejected, body: {}",
        resp_body
    );
    let json: serde_json::Value = serde_json::from_str(&resp_body).expect("valid JSON");
    assert_eq!(json["error"], "source content must not contain null bytes");
}

// --- Phase 4 endpoint tests ---

#[test]
fn well_known_tenor_returns_manifest() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let (status, body) = http_get(port, "/.well-known/tenor");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "manifest should return 200, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");

    // Must have the three manifest keys
    assert!(json.get("bundle").is_some(), "must have 'bundle' key");
    assert!(json.get("etag").is_some(), "must have 'etag' key");
    assert!(json.get("tenor").is_some(), "must have 'tenor' key");

    // bundle is a non-empty object
    let bundle = json["bundle"]
        .as_object()
        .expect("bundle must be an object");
    assert!(!bundle.is_empty(), "bundle must not be empty");

    // etag is a 64-char lowercase hex string (SHA-256)
    let etag = json["etag"].as_str().expect("etag must be a string");
    assert_eq!(
        etag.len(),
        64,
        "etag should be 64 hex chars, got {}",
        etag.len()
    );
    assert!(
        etag.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "etag must be lowercase hex, got: {}",
        etag
    );

    // tenor is a version string
    let tenor = json["tenor"].as_str().expect("tenor must be a string");
    assert!(!tenor.is_empty(), "tenor version must not be empty");
}

#[test]
fn well_known_tenor_etag_conditional_requests() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    // First request: 200 with ETag header
    let (status1, headers1, body1) = http_get_with_headers(port, "/.well-known/tenor", &[]);
    assert_eq!(status1, 200, "first request should be 200, body: {}", body1);

    let etag_header = extract_header(&headers1, "etag").expect("response must have ETag header");
    assert!(!etag_header.is_empty(), "ETag header must not be empty");

    // Second request: If-None-Match with correct ETag -> 304
    let (status2, _headers2, body2) = http_get_with_headers(
        port,
        "/.well-known/tenor",
        &[("If-None-Match", etag_header)],
    );
    assert_eq!(
        status2, 304,
        "matching If-None-Match should return 304, got body: {}",
        body2
    );
    assert!(
        body2.is_empty(),
        "304 response must have empty body, got: {}",
        body2
    );

    // Third request: If-None-Match with wrong ETag -> 200
    let (status3, _headers3, body3) = http_get_with_headers(
        port,
        "/.well-known/tenor",
        &[("If-None-Match", "\"wrong\"")],
    );

    child.kill().ok();
    child.wait().ok();

    assert_eq!(
        status3, 200,
        "mismatched If-None-Match should return 200, body: {}",
        body3
    );
}

#[test]
fn inspect_returns_structured_summary() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    let (status, body) = http_get(port, "/inspect");
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "inspect should return 200, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");

    // Verify structural keys
    assert!(json.get("facts").is_some(), "must have 'facts' key");
    assert!(json.get("entities").is_some(), "must have 'entities' key");
    assert!(json.get("rules").is_some(), "must have 'rules' key");
    assert!(json.get("personas").is_some(), "must have 'personas' key");
    assert!(
        json.get("operations").is_some(),
        "must have 'operations' key"
    );
    assert!(json.get("flows").is_some(), "must have 'flows' key");
    assert!(json.get("etag").is_some(), "must have 'etag' key");

    // All arrays should be non-empty for the SaaS contract
    let facts = json["facts"].as_array().expect("facts must be array");
    assert!(!facts.is_empty(), "facts should not be empty");

    let entities = json["entities"].as_array().expect("entities must be array");
    assert!(!entities.is_empty(), "entities should not be empty");

    let rules = json["rules"].as_array().expect("rules must be array");
    assert!(!rules.is_empty(), "rules should not be empty");

    let personas = json["personas"].as_array().expect("personas must be array");
    assert!(!personas.is_empty(), "personas should not be empty");

    let operations = json["operations"]
        .as_array()
        .expect("operations must be array");
    assert!(!operations.is_empty(), "operations should not be empty");

    let flows = json["flows"].as_array().expect("flows must be array");
    assert!(!flows.is_empty(), "flows should not be empty");

    // Spot-check: the Subscription entity should be present
    let has_subscription = entities.iter().any(|e| e["id"] == "Subscription");
    assert!(
        has_subscription,
        "Subscription entity should be in inspect output"
    );

    // Spot-check: the subscription_lifecycle flow should be present
    let has_flow = flows.iter().any(|f| f["id"] == "subscription_lifecycle");
    assert!(
        has_flow,
        "subscription_lifecycle flow should be in inspect output"
    );
}

#[test]
fn simulate_flow_valid_and_invalid() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    // Valid simulation: subscription_lifecycle with billing_system persona and good facts
    let simulate_body = serde_json::json!({
        "persona_id": "billing_system",
        "facts": {
            "current_seat_count": 15,
            "subscription_plan": "professional",
            "plan_features": {
                "max_seats": 50,
                "api_access": true,
                "sso_enabled": true,
                "custom_branding": false
            },
            "payment_ok": true,
            "account_age_days": 14
        }
    })
    .to_string();

    let (status, body) = http_post(
        port,
        "/flows/subscription_lifecycle/simulate",
        &simulate_body,
    );
    assert_eq!(status, 200, "simulate should succeed, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(json["simulation"], true, "simulation field must be true");
    assert_eq!(json["flow_id"], "subscription_lifecycle");
    assert!(json.get("outcome").is_some(), "must have 'outcome' key");
    assert!(json.get("path").is_some(), "must have 'path' key");
    assert!(
        json.get("would_transition").is_some(),
        "must have 'would_transition' key"
    );
    assert!(json.get("verdicts").is_some(), "must have 'verdicts' key");

    // Invalid flow_id: should return 404
    let (status_bad, body_bad) =
        http_post(port, "/flows/nonexistent_flow/simulate", &simulate_body);

    child.kill().ok();
    child.wait().ok();

    assert_eq!(
        status_bad, 404,
        "unknown flow should return 404, body: {}",
        body_bad
    );
    let json_bad: serde_json::Value = serde_json::from_str(&body_bad).expect("valid JSON");
    assert!(
        json_bad.get("error").is_some(),
        "error response must have 'error' key"
    );
}

#[test]
fn actions_returns_action_space() {
    let port = next_port();
    let mut child = start_server(port, &["domains/saas/saas_subscription.tenor"]);

    // billing_system persona with facts that make activation possible
    let actions_body = serde_json::json!({
        "persona_id": "billing_system",
        "facts": {
            "current_seat_count": 15,
            "subscription_plan": "professional",
            "plan_features": {
                "max_seats": 50,
                "api_access": true,
                "sso_enabled": true,
                "custom_branding": false
            },
            "payment_ok": true,
            "account_age_days": 14,
            "cancellation_requested": false
        },
        "entity_states": {
            "Subscription": "trial"
        }
    })
    .to_string();

    let (status, body) = http_post(port, "/actions", &actions_body);
    child.kill().ok();
    child.wait().ok();

    assert_eq!(status, 200, "actions should return 200, body: {}", body);
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");

    // ActionSpace must have actions and blocked_actions
    assert!(json.get("actions").is_some(), "must have 'actions' key");
    assert!(
        json.get("blocked_actions").is_some(),
        "must have 'blocked_actions' key"
    );

    let actions = json["actions"].as_array().expect("actions must be array");
    // billing_system has flows available — should have at least one action
    assert!(
        !actions.is_empty(),
        "billing_system should have available actions with these facts and entity states"
    );

    // Verify action structure
    let first_action = &actions[0];
    assert!(
        first_action.get("flow_id").is_some(),
        "action must have flow_id"
    );
    assert!(
        first_action.get("persona_id").is_some(),
        "action must have persona_id"
    );
    assert!(
        first_action.get("entry_operation_id").is_some(),
        "action must have entry_operation_id"
    );
}
