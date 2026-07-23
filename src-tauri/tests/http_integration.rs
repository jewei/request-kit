//! M1 integration tests: drive the executor directly against an in-process
//! fixture server (raw TcpListener + handcrafted HTTP/1.1 — well suited to
//! abrupt-close and slow-response cases). No external orchestration needed.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::PoisonError;
use std::thread;
use std::time::Duration;

use request_kit_lib::error::ErrorKind;
use request_kit_lib::http::executor;
use request_kit_lib::http::types::{HeaderEntry, SendBody, SendRequestPayload};
use request_kit_lib::http::validate;
use request_kit_lib::state::AppState;

const MIN_CAP: u64 = 1024 * 1024; // validation floor for max_body_bytes

fn spawn_fixture() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let addr = listener.local_addr().expect("fixture addr");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || handle(stream));
        }
    });
    addr
}

fn handle(mut stream: TcpStream) {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    // Read until end of request headers.
    while !buf.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => buf.push(byte[0]),
            _ => return,
        }
        if buf.len() > 65536 {
            return;
        }
    }
    let request = String::from_utf8_lossy(&buf);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();

    match path.as_str() {
        "/json" => {
            let body = br#"{"hello":"world"}"#;
            respond(
                &mut stream,
                200,
                "OK",
                &[("content-type", "application/json")],
                body,
            );
        }
        p if p.starts_with("/redirect/") => {
            let n: u32 = p.trim_start_matches("/redirect/").parse().unwrap_or(0);
            if n == 0 {
                respond(
                    &mut stream,
                    200,
                    "OK",
                    &[("content-type", "application/json")],
                    br#"{"redirected":true}"#,
                );
            } else {
                let location = format!("/redirect/{}", n - 1);
                respond(&mut stream, 302, "Found", &[("location", &location)], b"");
            }
        }
        "/gzip" => {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            let payload = br#"{"compressed":true}"#;
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(payload).unwrap();
            let gz = encoder.finish().unwrap();
            respond(
                &mut stream,
                200,
                "OK",
                &[
                    ("content-type", "application/json"),
                    ("content-encoding", "gzip"),
                ],
                &gz,
            );
        }
        "/dup" => {
            let head = "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\nx-dup: one\r\nx-dup: two\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok";
            let _ = stream.write_all(head.as_bytes());
        }
        "/big" => {
            // 4 MB body — four times the minimum download cap.
            let body = vec![b'a'; 4 * 1024 * 1024];
            respond(
                &mut stream,
                200,
                "OK",
                &[("content-type", "text/plain")],
                &body,
            );
        }
        "/binary" => {
            let bytes: Vec<u8> = (0u16..256).map(|v| v as u8).collect();
            respond(
                &mut stream,
                200,
                "OK",
                &[("content-type", "application/octet-stream")],
                &bytes,
            );
        }
        "/slow" => {
            thread::sleep(Duration::from_secs(10));
            respond(
                &mut stream,
                200,
                "OK",
                &[("content-type", "text/plain")],
                b"finally",
            );
        }
        "/close" => {
            // Claim a long body, write a fragment, then drop the socket.
            let head =
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 99999\r\n\r\npartial";
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.flush();
            // Dropping the stream aborts the connection mid-body.
        }
        _ => respond(&mut stream, 404, "Not Found", &[], b"{}"),
    }
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn payload(addr: SocketAddr, path: &str) -> SendRequestPayload {
    SendRequestPayload {
        execution_id: format!("exec-{path}-{}", uuid()),
        tab_id: "tab-1".into(),
        method: "GET".into(),
        url: format!("http://{addr}{path}"),
        headers: vec![],
        body: SendBody::None,
        timeout_ms: Some(5_000),
        follow_redirects: true,
        max_body_bytes: MIN_CAP,
    }
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[tokio::test]
async fn json_ok() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let p = payload(addr, "/json");
    let id = p.execution_id.clone();

    let res = executor::send(&state, p).await.expect("send ok");
    assert_eq!(res.status, 200);
    assert_eq!(res.status_text, "OK");
    assert_eq!(res.body.as_deref(), Some(r#"{"hello":"world"}"#));
    assert_eq!(res.content_type.as_deref(), Some("application/json"));
    assert_eq!(res.body_bytes, r#"{"hello":"world"}"#.len() as u64);
    assert!(!res.is_binary && !res.body_truncated && !res.download_capped);
    // Full bytes retained for save-to-file; abort map cleaned up.
    assert!(state.retention.contains(&id));
    assert!(state.abort_map.lock().unwrap().is_empty());
}

#[tokio::test]
async fn redirects_followed_and_not() {
    let addr = spawn_fixture();
    let state = AppState::default();

    let res = executor::send(&state, payload(addr, "/redirect/3"))
        .await
        .expect("redirect chain ok");
    assert_eq!(res.status, 200);
    assert!(res.final_url.ends_with("/redirect/0"));

    let mut no_follow = payload(addr, "/redirect/3");
    no_follow.follow_redirects = false;
    let res = executor::send(&state, no_follow)
        .await
        .expect("302 is a normal response");
    assert_eq!(res.status, 302);
}

#[tokio::test]
async fn gzip_is_decoded() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let res = executor::send(&state, payload(addr, "/gzip"))
        .await
        .expect("gzip ok");
    assert_eq!(res.body.as_deref(), Some(r#"{"compressed":true}"#));
    assert_eq!(res.body_bytes, r#"{"compressed":true}"#.len() as u64);
}

#[tokio::test]
async fn duplicate_headers_preserved() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let res = executor::send(&state, payload(addr, "/dup"))
        .await
        .expect("dup ok");
    let dups: Vec<&str> = res
        .headers
        .iter()
        .filter(|(name, _)| name == "x-dup")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(dups, vec!["one", "two"]);
}

#[tokio::test]
async fn oversize_body_is_capped() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let p = payload(addr, "/big");
    let id = p.execution_id.clone();
    let res = executor::send(&state, p).await.expect("capped ok");
    assert!(res.download_capped);
    assert_eq!(res.body_bytes, MIN_CAP);
    assert_eq!(state.retention.get(&id).unwrap().len() as u64, MIN_CAP);
}

#[tokio::test]
async fn binary_content_type_yields_no_text_body() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let res = executor::send(&state, payload(addr, "/binary"))
        .await
        .expect("binary ok");
    assert!(res.is_binary);
    assert!(res.body.is_none());
    assert_eq!(res.body_bytes, 256);
}

#[tokio::test]
async fn cancel_mid_flight() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let p = payload(addr, "/slow");
    let id = p.execution_id.clone();

    let send = executor::send(&state, p);
    tokio::pin!(send);
    // Let the request start, then cancel via the abort map (as the command does).
    let result = tokio::select! {
        res = &mut send => res,
        _ = tokio::time::sleep(Duration::from_millis(300)) => {
            if let Some(token) = state
                .abort_map
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&id)
            {
                token.cancel();
            }
            send.await
        }
    };

    let err = result.expect_err("cancelled");
    assert_eq!(err.kind, ErrorKind::Cancelled);
    assert!(
        state.abort_map.lock().unwrap().is_empty(),
        "abort map cleaned"
    );
    assert!(
        !state.retention.contains(&id),
        "cancelled send retains nothing"
    );
}

#[tokio::test]
async fn abrupt_close_is_classified_not_panicking() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let err = executor::send(&state, payload(addr, "/close"))
        .await
        .expect_err("abrupt close fails");
    assert!(
        matches!(
            err.kind,
            ErrorKind::Connection | ErrorKind::Io | ErrorKind::Unknown
        ),
        "unexpected kind: {:?}",
        err.kind
    );
}

#[tokio::test]
async fn timeout_is_classified() {
    let addr = spawn_fixture();
    let state = AppState::default();
    let mut p = payload(addr, "/slow");
    p.timeout_ms = Some(300);
    let err = executor::send(&state, p).await.expect_err("times out");
    assert_eq!(err.kind, ErrorKind::Timeout);
}

#[tokio::test]
async fn invalid_scheme_rejected() {
    let state = AppState::default();
    let mut p = payload("127.0.0.1:1".parse().unwrap(), "/x");
    p.url = "ftp://example.com/file".into();
    let err = executor::send(&state, p).await.expect_err("ftp rejected");
    assert_eq!(err.kind, ErrorKind::InvalidUrl);
}

#[tokio::test]
async fn maintenance_mode_rejects_sends() {
    let addr = spawn_fixture();
    let state = AppState::default();
    *state.mode.lock().unwrap() = request_kit_lib::state::AppMode::ImportApplying;
    let err = executor::send(&state, payload(addr, "/json"))
        .await
        .expect_err("maintenance rejects");
    assert_eq!(err.kind, ErrorKind::MaintenanceInProgress);
}

// ---- validation limits (no network) ----

fn valid_payload() -> SendRequestPayload {
    SendRequestPayload {
        execution_id: "e1".into(),
        tab_id: "t1".into(),
        method: "GET".into(),
        url: "https://example.com/".into(),
        headers: vec![],
        body: SendBody::None,
        timeout_ms: None,
        follow_redirects: true,
        max_body_bytes: MIN_CAP,
    }
}

#[test]
fn validation_limits() {
    // Method allowlist.
    let mut p = valid_payload();
    p.method = "BREW".into();
    assert_eq!(
        validate::validate_payload(&p).unwrap_err().kind,
        ErrorKind::Validation
    );

    // Header count limit.
    let mut p = valid_payload();
    p.headers = (0..201)
        .map(|i| HeaderEntry {
            name: format!("x-h-{i}"),
            value: "v".into(),
        })
        .collect();
    assert!(validate::validate_payload(&p).is_err());

    // Aggregate header bytes.
    let mut p = valid_payload();
    p.headers = vec![HeaderEntry {
        name: "x-big".into(),
        value: "v".repeat(300 * 1024),
    }];
    assert!(validate::validate_payload(&p).is_err());

    // Timeout bounds.
    let mut p = valid_payload();
    p.timeout_ms = Some(0);
    assert!(validate::validate_payload(&p).is_err());
    p.timeout_ms = Some(600_001);
    assert!(validate::validate_payload(&p).is_err());
    p.timeout_ms = Some(600_000);
    assert!(validate::validate_payload(&p).is_ok());

    // max_body_bytes bounds.
    let mut p = valid_payload();
    p.max_body_bytes = MIN_CAP - 1;
    assert!(validate::validate_payload(&p).is_err());
    p.max_body_bytes = 100 * 1024 * 1024 + 1;
    assert!(validate::validate_payload(&p).is_err());

    // Text body cap.
    let mut p = valid_payload();
    p.body = SendBody::Text {
        content: "a".repeat(10 * 1024 * 1024 + 1),
    };
    assert!(validate::validate_payload(&p).is_err());

    // Multipart/file rejected until M4.
    let mut p = valid_payload();
    p.body = SendBody::Multipart { parts: vec![] };
    assert!(validate::validate_payload(&p).is_err());
    let mut p = valid_payload();
    p.body = SendBody::File {
        path: "/tmp/x".into(),
    };
    assert!(validate::validate_payload(&p).is_err());

    // Ids.
    let mut p = valid_payload();
    p.execution_id = String::new();
    assert!(validate::validate_payload(&p).is_err());
    let mut p = valid_payload();
    p.tab_id = "t".repeat(129);
    assert!(validate::validate_payload(&p).is_err());
}

// ---- retention FIFO (no network) ----

#[test]
fn retention_fifo_budgets_and_idempotent_release() {
    use request_kit_lib::http::retain::Retention;

    // Entry-count budget: 11 inserts → oldest evicted.
    let r = Retention::new();
    for i in 0..11 {
        r.insert(&format!("id-{i}"), vec![0u8; 8]);
    }
    assert_eq!(r.len(), 10);
    assert!(!r.contains("id-0"), "oldest evicted");
    assert!(r.contains("id-10"));

    // Byte budget: three 20 MB entries exceed 50 MB → oldest evicted.
    let r = Retention::new();
    r.insert("a", vec![0u8; 20 * 1024 * 1024]);
    r.insert("b", vec![0u8; 20 * 1024 * 1024]);
    r.insert("c", vec![0u8; 20 * 1024 * 1024]);
    assert!(!r.contains("a"));
    assert!(r.contains("b") && r.contains("c"));
    assert!(r.total_bytes() <= 50 * 1024 * 1024);

    // Idempotent release.
    r.release("b");
    r.release("b");
    assert!(!r.contains("b"));

    // Replacing an id does not double-count bytes.
    let r = Retention::new();
    r.insert("x", vec![0u8; 1000]);
    r.insert("x", vec![0u8; 500]);
    assert_eq!(r.total_bytes(), 500);
    assert_eq!(r.len(), 1);
}

// ---- error classification + redaction (no network) ----

#[test]
fn classification_of_constructed_chains() {
    use request_kit_lib::http::error_map::classify;
    use std::fmt;

    #[derive(Debug)]
    struct Wrapper {
        message: &'static str,
        source: Option<std::io::Error>,
    }
    impl fmt::Display for Wrapper {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }
    impl std::error::Error for Wrapper {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.source.as_ref().map(|e| e as _)
        }
    }

    let refused = Wrapper {
        message: "transport failed",
        source: Some(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        )),
    };
    assert_eq!(classify(&refused), ErrorKind::Connection);

    let dns = Wrapper {
        message: "dns error: no records",
        source: None,
    };
    assert_eq!(classify(&dns), ErrorKind::Dns);

    let tls = Wrapper {
        message: "rustls handshake failure",
        source: None,
    };
    assert_eq!(classify(&tls), ErrorKind::Tls);

    let unknown = Wrapper {
        message: "something odd",
        source: None,
    };
    assert_eq!(classify(&unknown), ErrorKind::Unknown);
}

#[test]
fn url_redaction() {
    use request_kit_lib::http::error_map::redact_url_like;

    let input = "request to https://api.example.com/v1/users?apiKey=SECRET123&limit=10&flag failed";
    let out = redact_url_like(input);
    assert!(!out.contains("SECRET123"));
    assert!(out.contains("apiKey=<redacted>"));
    assert!(out.contains("limit=<redacted>"));
    assert!(out.contains("&flag"), "valueless keys kept: {out}");

    let creds = "error at https://user:hunter2@example.com/path";
    let out = redact_url_like(creds);
    assert!(!out.contains("hunter2"));
    assert!(out.contains("<redacted>@example.com"));

    // Non-URL text passes through untouched.
    assert_eq!(redact_url_like("plain message"), "plain message");
}
