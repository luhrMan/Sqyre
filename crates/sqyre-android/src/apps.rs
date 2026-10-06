//! Launchable apps reported by the shell as `package\tlabel` lines.

/// One app the launcher can start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchableApp {
    pub package: String,
    /// User-visible app label; empty when Android has none.
    pub label: String,
}

impl LaunchableApp {
    /// Last package segment (`com.android.chrome` → `chrome`), the closest thing to a process name.
    pub fn short_name(&self) -> &str {
        self.package.rsplit('.').next().unwrap_or(&self.package)
    }
}

/// Parse one `package\tlabel` line. Blank or package-less lines are `None`.
pub fn parse_app_line(line: &str) -> Option<LaunchableApp> {
    let (package, label) = line.split_once('\t').unwrap_or((line, ""));
    let package = package.trim();
    if package.is_empty() {
        return None;
    }
    Some(LaunchableApp {
        package: package.to_string(),
        label: label.trim().to_string(),
    })
}

/// Parse the shell's app list: one app per package, sorted by label then package
/// (case-insensitive). Apps with several launcher activities keep the first label.
pub fn parse_app_list(text: &str) -> Vec<LaunchableApp> {
    let mut apps: Vec<LaunchableApp> = Vec::new();
    for app in text.lines().filter_map(parse_app_line) {
        if !apps.iter().any(|a| a.package == app.package) {
            apps.push(app);
        }
    }
    apps.sort_by_cached_key(|a| (a.label.to_lowercase(), a.package.to_lowercase()));
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(package: &str, label: &str) -> LaunchableApp {
        LaunchableApp {
            package: package.into(),
            label: label.into(),
        }
    }

    #[test]
    fn parses_package_and_label() {
        assert_eq!(
            parse_app_line("com.android.chrome\tChrome"),
            Some(app("com.android.chrome", "Chrome"))
        );
        assert_eq!(
            parse_app_line("  org.example \t  Example App \r"),
            Some(app("org.example", "Example App"))
        );
    }

    #[test]
    fn missing_label_is_empty() {
        assert_eq!(parse_app_line("org.example"), Some(app("org.example", "")));
        assert_eq!(
            parse_app_line("org.example\t"),
            Some(app("org.example", ""))
        );
    }

    #[test]
    fn blank_or_packageless_lines_are_skipped() {
        assert_eq!(parse_app_line(""), None);
        assert_eq!(parse_app_line("   "), None);
        assert_eq!(parse_app_line("\tLabel only"), None);
    }

    #[test]
    fn list_dedupes_and_sorts_by_label() {
        let text = "com.b\tbeta\n\ncom.a\tAlpha\ncom.b\tBeta Two\ncom.c\talpha\n";
        assert_eq!(
            parse_app_list(text),
            vec![
                app("com.a", "Alpha"),
                app("com.c", "alpha"),
                app("com.b", "beta")
            ]
        );
    }

    #[test]
    fn empty_list_is_empty() {
        assert!(parse_app_list("").is_empty());
    }

    #[test]
    fn short_name_is_the_last_segment() {
        assert_eq!(app("com.android.chrome", "").short_name(), "chrome");
        assert_eq!(app("single", "").short_name(), "single");
    }
}
