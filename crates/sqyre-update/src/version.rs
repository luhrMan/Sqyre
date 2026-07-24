//! Parse and compare Sqyre release tags (`vYYYY.MM.DD` or `vYYYY.MM.DD.HHMM`).

/// Dev sentinel embedded when neither `RELEASE_VERSION` nor a `VERSION` file is present.
pub const DEV_SENTINEL: &str = "0.0.0-dev";

/// `(year, month, day, hhmm)` — `hhmm` is `0` when the tag has no time suffix.
pub type VersionParts = (u32, u32, u32, u32);

pub fn is_dev_sentinel(version: &str) -> bool {
    let v = strip_v(version);
    v == DEV_SENTINEL || v.ends_with("-dev")
}

pub fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
}

/// Parse `YYYY.MM.DD` or `YYYY.MM.DD.HHMM` (optional leading `v`).
pub fn parse_release_version(tag: &str) -> Option<VersionParts> {
    let s = strip_v(tag);
    if is_dev_sentinel(s) {
        return None;
    }
    let mut parts = s.split('.');
    let year: u32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    let hhmm: u32 = match parts.next() {
        Some(t) => t.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day, hhmm))
}

/// True when `remote` is strictly newer than `current`.
pub fn version_newer(remote: &VersionParts, current: &VersionParts) -> bool {
    remote > current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_date_and_datetime() {
        assert_eq!(parse_release_version("v2026.07.23"), Some((2026, 7, 23, 0)));
        assert_eq!(
            parse_release_version("2026.07.23.1430"),
            Some((2026, 7, 23, 1430))
        );
        assert!(parse_release_version("0.0.0-dev").is_none());
        assert!(parse_release_version("v1.2").is_none());
    }

    #[test]
    fn compares_versions() {
        let a = parse_release_version("v2026.07.22").unwrap();
        let b = parse_release_version("v2026.07.23").unwrap();
        let c = parse_release_version("v2026.07.23.0100").unwrap();
        assert!(version_newer(&b, &a));
        assert!(!version_newer(&a, &b));
        assert!(version_newer(&c, &b));
        assert!(!version_newer(&b, &c));
        assert!(!version_newer(&b, &b));
    }

    #[test]
    fn detects_dev() {
        assert!(is_dev_sentinel("0.0.0-dev"));
        assert!(is_dev_sentinel("v0.0.0-dev"));
        assert!(!is_dev_sentinel("2026.07.23"));
    }
}
