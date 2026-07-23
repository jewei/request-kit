//! Error classification and redaction. `classify` walks the `source()` chain
//! of a transport error and maps it onto the typed `ErrorKind` taxonomy;
//! `redact_url_like` strips credentials and query values from any URL-looking
//! substrings so that `AppError.detail` never leaks a resolved secret.

use std::error::Error;

use crate::error::ErrorKind;

/// Best-effort classification by walking the error source chain. DNS detection
/// is deliberately conservative (resolver errors differ per OS): when a
/// connect error is ambiguous we fall back to `Connection`, never fabricating
/// specificity; anything else unrecognized is `Unknown`.
pub fn classify(err: &(dyn Error + 'static)) -> ErrorKind {
    let mut connect_error_seen = false;
    let mut current: Option<&(dyn Error + 'static)> = Some(err);
    while let Some(e) = current {
        if let Some(re) = e.downcast_ref::<reqwest::Error>() {
            if re.is_timeout() {
                return ErrorKind::Timeout;
            }
            if re.is_builder() {
                return ErrorKind::InvalidUrl;
            }
            if re.is_connect() {
                connect_error_seen = true;
            }
        }
        // rustls is not a direct dependency, so a typed downcast is not
        // available; its errors reliably mention "rustls" in Display output.
        let message = e.to_string().to_ascii_lowercase();
        if message.contains("rustls") {
            return ErrorKind::Tls;
        }
        if message.contains("dns error") || message.contains("failed to lookup address") {
            return ErrorKind::Dns;
        }
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind as IoKind;
            match io.kind() {
                IoKind::ConnectionRefused | IoKind::ConnectionReset | IoKind::BrokenPipe => {
                    return ErrorKind::Connection;
                }
                IoKind::TimedOut => return ErrorKind::Timeout,
                _ => {}
            }
        }
        current = e.source();
    }
    if connect_error_seen {
        ErrorKind::Connection
    } else {
        ErrorKind::Unknown
    }
}

/// Redacts query values and userinfo credentials in every URL-looking
/// substring (`scheme://…`) of `text`. Query keys are kept; values become
/// `<redacted>`. `AppError.detail` must only ever contain output of this
/// function.
pub fn redact_url_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while let Some(rel) = text[i..].find("://") {
        let sep = i + rel;
        // Walk back over the scheme characters (ASCII, so byte-safe).
        let mut start = sep;
        while start > i {
            let c = bytes[start - 1];
            if c.is_ascii_alphanumeric() || c == b'+' || c == b'-' || c == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        // Extend forward to the end of the URL-looking span.
        let mut end = sep + 3;
        while end < bytes.len() && !is_url_terminator(bytes[end]) {
            end += 1;
        }
        out.push_str(&text[i..start]);
        out.push_str(&redact_one_url(&text[start..end]));
        i = end;
    }
    out.push_str(&text[i..]);
    out
}

fn is_url_terminator(b: u8) -> bool {
    b.is_ascii_whitespace() || matches!(b, b'"' | b'\'' | b'<' | b'>' | b'(' | b')' | b'`')
}

fn redact_one_url(url: &str) -> String {
    let (without_fragment, fragment) = match url.find('#') {
        Some(pos) => (&url[..pos], Some(&url[pos..])),
        None => (url, None),
    };

    let mut redacted = String::with_capacity(url.len());

    let after_scheme = without_fragment
        .find("://")
        .map(|p| p + 3)
        .unwrap_or_default();
    let authority_end = without_fragment[after_scheme..]
        .find(['/', '?'])
        .map(|p| after_scheme + p)
        .unwrap_or(without_fragment.len());
    let authority = &without_fragment[after_scheme..authority_end];

    if let Some(at) = authority.rfind('@') {
        // Userinfo (user or user:password) is always a credential.
        redacted.push_str(&without_fragment[..after_scheme]);
        redacted.push_str("<redacted>@");
        redacted.push_str(&authority[at + 1..]);
    } else {
        redacted.push_str(&without_fragment[..authority_end]);
    }

    let rest = &without_fragment[authority_end..];
    if let Some(q) = rest.find('?') {
        redacted.push_str(&rest[..=q]);
        let query = &rest[q + 1..];
        let mut first = true;
        for segment in query.split('&') {
            if !first {
                redacted.push('&');
            }
            first = false;
            match segment.find('=') {
                Some(eq) => {
                    redacted.push_str(&segment[..=eq]);
                    redacted.push_str("<redacted>");
                }
                None => redacted.push_str(segment),
            }
        }
    } else {
        redacted.push_str(rest);
    }

    if let Some(f) = fragment {
        redacted.push_str(f);
    }
    redacted
}

/// Joins an error's Display output with its full source chain — the input to
/// `redact_url_like` when building `AppError.detail`.
pub fn chain_text(err: &(dyn Error + 'static)) -> String {
    let mut parts = vec![err.to_string()];
    let mut current = err.source();
    while let Some(e) = current {
        parts.push(e.to_string());
        current = e.source();
    }
    parts.join(": ")
}
