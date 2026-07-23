use std::collections::HashMap;
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

use crate::http::clients::HttpClients;
use crate::http::retain::Retention;
use crate::storage::workspace::Workspace;

/// Global application mode. Sends and storage mutations are rejected while an
/// import is being applied or recovered (see PLAN.md, import semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    #[allow(dead_code)] // used from M3b (import staging)
    ImportStaging,
    #[allow(dead_code)] // used from M3b (import apply)
    ImportApplying,
    #[allow(dead_code)] // used from M3b (startup recovery)
    Recovery,
}

#[derive(Default)]
pub struct AppState {
    /// In-flight sends, keyed by execution id.
    pub abort_map: Mutex<HashMap<String, CancellationToken>>,
    pub mode: Mutex<AppMode>,
    pub http_clients: HttpClients,
    pub retention: Retention,
    /// Loaded on the first `load_workspace`; None until then.
    pub workspace: Mutex<Option<Workspace>>,
}
