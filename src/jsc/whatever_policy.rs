use std::path::{Component, Path, PathBuf};

pub(crate) const API_MODULE: &str = "@whatever-engine/api";

pub(crate) struct ModulePolicy {
    root: PathBuf,
}

impl ModulePolicy {
    pub(crate) fn new(root: &Path) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(std::io::Error::other("mod root must be a directory"));
        }
        Ok(Self { root })
    }

    pub(crate) fn allows_specifier(&self, specifier: &str) -> bool {
        if specifier == API_MODULE {
            return true;
        }
        if specifier.contains(['\0', '?', '#']) || has_node_modules(Path::new(specifier)) {
            return false;
        }
        specifier.starts_with("./")
            || specifier.starts_with("../")
            || (cfg!(windows) && (specifier.starts_with(".\\") || specifier.starts_with("..\\")))
            || self.allows_resolved(specifier)
    }

    pub(crate) fn allows_resolved(&self, specifier: &str) -> bool {
        if specifier == API_MODULE {
            return true;
        }
        if specifier.contains(['\0', '?', '#']) {
            return false;
        }
        let path = Path::new(specifier);
        if !path.is_absolute() || has_node_modules(path) {
            return false;
        }
        let Ok(resolved) = path.canonicalize() else {
            return false;
        };
        resolved.starts_with(&self.root)
            && resolved.is_file()
            && !has_node_modules(&resolved)
            && matches!(
                resolved.extension().and_then(|ext| ext.to_str()),
                Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" | "json")
            )
    }
}

fn has_node_modules(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_str().is_some_and(|name| name.eq_ignore_ascii_case("node_modules")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "whatever-policy-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(path.join("mod/node_modules/package")).expect("fixture");
            std::fs::create_dir_all(path.join("mod-other")).expect("fixture");
            for name in [
                "mod/main.ts",
                "mod/data.json",
                "mod/addon.node",
                "mod/node_modules/package/index.js",
                "mod-other/secret.js",
            ] {
                std::fs::write(path.join(name), "").expect("fixture");
            }
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn packages_and_other_namespaces_are_denied() {
        let fixture = Fixture::new();
        let policy = ModulePolicy::new(&fixture.0.join("mod")).expect("policy");
        for name in [
            "fs",
            "node:fs",
            "bun",
            "bun:ffi",
            "lodash",
            "@scope/package",
            "#alias",
            "data:text/javascript,export default 1",
            "blob:uuid",
            "https://example.invalid/mod.js",
            "@whatever-engine/api/internal",
            "./node_modules/package/index.js",
        ] {
            assert!(!policy.allows_specifier(name), "{name}");
        }
        assert!(policy.allows_specifier(API_MODULE));
        assert!(policy.allows_specifier("./main.ts"));
        assert!(policy.allows_specifier("../sibling.ts"));
    }

    #[test]
    fn resolved_modules_must_be_source_files_inside_the_mod() {
        let fixture = Fixture::new();
        let policy = ModulePolicy::new(&fixture.0.join("mod")).expect("policy");
        for name in ["mod/main.ts", "mod/data.json"] {
            assert!(policy.allows_resolved(fixture.0.join(name).to_str().expect("path")));
        }
        for name in [
            "mod/addon.node",
            "mod/missing.ts",
            "mod/node_modules/package/index.js",
            "mod-other/secret.js",
            "mod/../mod-other/secret.js",
        ] {
            assert!(
                !policy.allows_resolved(fixture.0.join(name).to_str().expect("path")),
                "{name}"
            );
        }
        assert!(!policy.allows_resolved("./main.ts"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_the_mod_or_hide_node_modules() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink(fixture.0.join("mod-other"), fixture.0.join("mod/escape"))
            .expect("symlink");
        std::os::unix::fs::symlink(
            fixture.0.join("mod/node_modules"),
            fixture.0.join("mod/packages"),
        )
        .expect("symlink");
        let policy = ModulePolicy::new(&fixture.0.join("mod")).expect("policy");
        assert!(
            !policy.allows_resolved(
                fixture
                    .0
                    .join("mod/escape/secret.js")
                    .to_str()
                    .expect("path")
            )
        );
        assert!(
            !policy.allows_resolved(
                fixture
                    .0
                    .join("mod/packages/package/index.js")
                    .to_str()
                    .expect("path")
            )
        );
    }
}
