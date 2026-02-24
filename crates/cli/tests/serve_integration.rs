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

/// Parse an HTTP response into (status_code, body).
fn parse_http_response(response: &str) -> (u16, String) {
    let parts: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
    let headers = parts.first().unwrap_or(&"");
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

    (status, body)
}

/// Decode chunked transfer encoding.
fn decode_chunked(data: &str) -> String {
    let mut result = String::new();
    let mut remaining = data;

    loop {
        // Find the chunk size line
        let line_end = match remaining.find("\r\n") {
            Some(pos) => pos,
            None => break,
        };
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
