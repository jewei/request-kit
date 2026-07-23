//! Filesystem-safe slug generation (PLAN.md, platform targets). Slugs must be
//! valid on both APFS and NTFS: Windows-reserved characters and names are
//! stripped/escaped, and collisions get numeric suffixes.

/// Windows reserved device stems (case-insensitive), unusable as file names.
const RESERVED_STEMS: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Turns a display name into a single filesystem-safe path component (no
/// extension). Never returns an empty string.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let unsafe_char = matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            || ch.is_control()
            || ch.is_whitespace();
        out.push(if unsafe_char { '-' } else { ch });
    }
    out = out.to_lowercase();

    // Collapse runs of '-' and trim separators/dots/spaces from both ends.
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let trimmed = out.trim_matches(|c| c == '-' || c == '.' || c == ' ');
    let mut slug = trimmed.to_string();

    if slug.is_empty() {
        return "untitled".to_string();
    }

    // A reserved device stem (the part before the first '.') is unusable.
    let stem = slug.split('.').next().unwrap_or(&slug).to_uppercase();
    if RESERVED_STEMS.contains(&stem.as_str()) {
        slug.push_str("-file");
    }
    slug
}

/// Applies `-2`, `-3`, … to `base` until `taken(candidate)` is false.
pub fn unique_slug(base: &str, taken: &dyn Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_reserved_chars() {
        assert_eq!(slugify("a/b:c?"), "a-b-c");
    }

    #[test]
    fn trims_trailing_dots_spaces() {
        assert_eq!(slugify("name. "), "name");
    }

    #[test]
    fn collapses_separator_runs() {
        assert_eq!(slugify("a   b"), "a-b");
    }

    #[test]
    fn reserved_stem_gets_suffixed() {
        // slugify receives bare display names; the `.json` extension is added
        // by callers, so a reserved stem is simply made safe.
        assert_eq!(slugify("CON"), "con-file");
        assert_eq!(slugify("prn"), "prn-file");
    }

    #[test]
    fn empty_becomes_untitled() {
        assert_eq!(slugify("  "), "untitled");
        assert_eq!(slugify("///"), "untitled");
    }

    #[test]
    fn collisions_suffix() {
        let taken = |s: &str| s == "req" || s == "req-2";
        assert_eq!(unique_slug("req", &taken), "req-3");
    }

    #[test]
    fn no_collision_returns_base() {
        let taken = |_: &str| false;
        assert_eq!(unique_slug("req", &taken), "req");
    }
}
