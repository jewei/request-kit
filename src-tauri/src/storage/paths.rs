use std::path::PathBuf;

use crate::error::{AppError, ErrorKind};

/// Resolves the app-managed storage root: `~/.request-kit`.
///
/// User decision (PLAN.md): home-relative, human-readable, Bruno-style — not
/// the platform app-data dir. Never build this path with hardcoded separators.
pub fn storage_root() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::new(ErrorKind::Io, "could not resolve the home directory"))?;
    Ok(home.join(".request-kit"))
}

/// Creates the storage root (and mode 0700 on Unix) if it does not exist.
pub fn ensure_storage_root() -> Result<PathBuf, AppError> {
    let root = storage_root()?;
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .map_err(|e| AppError::io(format!("could not create storage root: {e}")))?;
        #[cfg(unix)]
        {
            use std::fs::Permissions;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, Permissions::from_mode(0o700))
                .map_err(|e| AppError::io(format!("could not set storage permissions: {e}")))?;
        }
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_root_is_home_relative() {
        let root = storage_root().unwrap();
        assert!(root.ends_with(".request-kit"));
        assert!(root.is_absolute());
    }
}
