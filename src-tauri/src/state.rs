use std::collections::HashMap;
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

/// Global application mode. Sends and storage mutations are rejected while an
/// import is being applied or recovered (see PLAN.md, import semantics).
#[allow(dead_code)] // wired up from M1 (sends) and M3b (import)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    ImportStaging,
    ImportApplying,
    Recovery,
}

#[derive(Default)]
pub struct AppState {
    /// In-flight sends, keyed by execution id.
    #[allow(dead_code)] // used from M1 (send/cancel)
    pub abort_map: Mutex<HashMap<String, CancellationToken>>,
    #[allow(dead_code)] // used from M1 (send guard) and M3b (import)
    pub mode: Mutex<AppMode>,
}
