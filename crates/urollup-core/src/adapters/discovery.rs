//! Default persistent-log locations and environment-variable precedence.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Why a set of agent-log roots was selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootOrigin {
    /// The urollup-specific path-list override.
    Override,
    /// The agent's own configuration variable.
    NativeVariable,
    /// Conventional locations below the user's home or XDG configuration directory.
    Default,
}

/// The roots selected for one adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootSelection {
    /// Roots in deterministic precedence order.
    pub roots: Vec<PathBuf>,
    /// What selected them.
    pub origin: RootOrigin,
}

impl RootSelection {
    /// Whether a missing selected root is an error rather than an absent default.
    pub const fn missing_is_error(&self) -> bool {
        !matches!(self.origin, RootOrigin::Default)
    }
}

/// The small environment surface that controls adapter discovery.
///
/// Keeping it as data makes precedence tests hermetic and avoids mutating process-wide
/// environment variables in parallel tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiscoveryEnvironment {
    /// The user's home directory, when established.
    pub home: Option<PathBuf>,
    variables: BTreeMap<String, OsString>,
}

impl DiscoveryEnvironment {
    /// Captures the relevant values from the current process.
    pub fn from_process() -> Self {
        let variables = [
            "UROLLUP_CLAUDE_CONFIG_DIRS",
            "CLAUDE_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            "UROLLUP_CODEX_HOMES",
            "CODEX_HOME",
        ]
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name.to_owned(), value)))
        .collect();
        let home =
            std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
        Self { home, variables }
    }

    /// Builds a discovery environment from explicit values.
    pub fn new(
        home: Option<PathBuf>,
        variables: impl IntoIterator<Item = (impl Into<String>, OsString)>,
    ) -> Self {
        Self {
            home,
            variables: variables.into_iter().map(|(name, value)| (name.into(), value)).collect(),
        }
    }

    /// Selects Claude Code transcript roots.
    ///
    /// Both Claude variables name configuration directories, so their `projects/`
    /// children are the roots walked by the adapter.
    pub fn claude_project_roots(&self) -> RootSelection {
        if let Some(value) =
            self.variables.get("UROLLUP_CLAUDE_CONFIG_DIRS").filter(|v| !v.is_empty())
        {
            return RootSelection {
                roots: config_paths(value).map(|path| path.join("projects")).collect(),
                origin: RootOrigin::Override,
            };
        }
        if let Some(value) = self.variables.get("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
            return RootSelection {
                roots: [PathBuf::from(value).join("projects")].into_iter().collect(),
                origin: RootOrigin::NativeVariable,
            };
        }

        let mut roots = Vec::new();
        if let Some(home) = &self.home {
            roots.push(home.join(".claude/projects"));
        }
        if let Some(xdg) = self.variables.get("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
            roots.push(PathBuf::from(xdg).join("claude/projects"));
        } else if let Some(home) = &self.home {
            roots.push(home.join(".config/claude/projects"));
        }
        roots.sort();
        roots.dedup();
        RootSelection { roots, origin: RootOrigin::Default }
    }

    /// Selects Codex homes. Each home contains `sessions/` and
    /// `archived_sessions/`, which are walked together so archived renames reconcile.
    pub fn codex_homes(&self) -> RootSelection {
        if let Some(value) = self.variables.get("UROLLUP_CODEX_HOMES").filter(|v| !v.is_empty()) {
            return RootSelection {
                roots: config_paths(value).collect(),
                origin: RootOrigin::Override,
            };
        }
        if let Some(value) = self.variables.get("CODEX_HOME").filter(|v| !v.is_empty()) {
            return RootSelection {
                roots: [PathBuf::from(value)].into_iter().collect(),
                origin: RootOrigin::NativeVariable,
            };
        }
        RootSelection {
            roots: self.home.iter().map(|home| home.join(".codex")).collect(),
            origin: RootOrigin::Default,
        }
    }
}

fn config_paths(value: &OsStr) -> impl Iterator<Item = PathBuf> + '_ {
    std::env::split_paths(value).filter(|path| !path.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{DiscoveryEnvironment, RootOrigin};

    #[test]
    fn claude_override_path_list_wins() {
        let override_value = std::env::join_paths(["/one", "/two"]).unwrap();
        let environment = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [
                ("UROLLUP_CLAUDE_CONFIG_DIRS", override_value),
                ("CLAUDE_CONFIG_DIR", OsString::from("/native")),
            ],
        );

        let selected = environment.claude_project_roots();

        assert_eq!(selected.origin, RootOrigin::Override);
        assert_eq!(
            selected.roots,
            [PathBuf::from("/one/projects"), PathBuf::from("/two/projects")]
        );
        assert!(selected.missing_is_error());
    }

    #[test]
    fn claude_defaults_include_home_and_xdg_locations() {
        let environment = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [("XDG_CONFIG_HOME", OsString::from("/xdg"))],
        );

        let selected = environment.claude_project_roots();

        assert_eq!(selected.origin, RootOrigin::Default);
        assert_eq!(
            selected.roots,
            [
                PathBuf::from("/home/example/.claude/projects"),
                PathBuf::from("/xdg/claude/projects")
            ]
        );
        assert!(!selected.missing_is_error());
    }

    #[test]
    fn claude_native_variable_is_one_configuration_directory() {
        let native_value = std::env::join_paths(["/one", "/two"]).unwrap();
        let environment = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [("CLAUDE_CONFIG_DIR", native_value.clone())],
        );

        let selected = environment.claude_project_roots();

        assert_eq!(selected.origin, RootOrigin::NativeVariable);
        assert_eq!(selected.roots, [PathBuf::from(native_value).join("projects")]);
    }

    #[test]
    fn empty_agent_variables_do_not_hide_defaults() {
        let environment = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [
                ("UROLLUP_CLAUDE_CONFIG_DIRS", OsString::new()),
                ("CLAUDE_CONFIG_DIR", OsString::new()),
                ("UROLLUP_CODEX_HOMES", OsString::new()),
                ("CODEX_HOME", OsString::new()),
            ],
        );

        assert_eq!(environment.claude_project_roots().origin, RootOrigin::Default);
        assert_eq!(environment.codex_homes().origin, RootOrigin::Default);
    }

    #[test]
    fn codex_override_wins_and_native_variable_is_scalar() {
        let override_value = std::env::join_paths(["/one", "/two"]).unwrap();
        let overridden = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [("UROLLUP_CODEX_HOMES", override_value), ("CODEX_HOME", OsString::from("/native"))],
        );
        assert_eq!(overridden.codex_homes().roots, [PathBuf::from("/one"), PathBuf::from("/two")]);

        let native = DiscoveryEnvironment::new(
            Some(PathBuf::from("/home/example")),
            [("CODEX_HOME", OsString::from("/native"))],
        );
        assert_eq!(native.codex_homes().origin, RootOrigin::NativeVariable);
        assert_eq!(native.codex_homes().roots, [PathBuf::from("/native")]);
    }
}
