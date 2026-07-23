//! The two shared reqwest clients, built once at startup and reused for every
//! send (each client owns a connection pool — per reqwest's own guidance).

use reqwest::redirect::Policy;
use reqwest::Client;

/// Redirect limit is fixed at 10 (PLAN.md: not a per-request pipeline input).
const REDIRECT_LIMIT: usize = 10;

pub struct HttpClients {
    pub follow_redirects: Client,
    pub no_redirects: Client,
}

impl HttpClients {
    pub fn new() -> Self {
        Self {
            follow_redirects: build(Policy::limited(REDIRECT_LIMIT)),
            no_redirects: build(Policy::none()),
        }
    }
}

impl Default for HttpClients {
    fn default() -> Self {
        Self::new()
    }
}

fn build(policy: Policy) -> Client {
    Client::builder()
        .redirect(policy)
        // Behavioral guarantee: proxy support is out of scope, and omitting the
        // `system-proxy` cargo feature alone is not enough — features are
        // additive across the dependency graph.
        .no_proxy()
        // No cookie store; never `danger_accept_invalid_certs`.
        .build()
        // Startup-fatal: without a TLS backend the app cannot function.
        .expect("failed to construct the shared HTTP client")
}
