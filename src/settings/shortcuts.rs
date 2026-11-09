//! User-owned overrides, separate from the frequently saved workspace settings.
use crate::keymap::{Overrides, defaults, normalize};
use anyhow::Result;
use rexrs::api::client::Diagnostic;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Shortcuts {
    pub path: PathBuf,
    pub overrides: Overrides,
    /// Taken once by the UI; Problems keeps the diagnostics after dismissal.
    pub notice: Option<String>,
    pub imported: Option<usize>,
    stamp: Option<(std::time::SystemTime, u64)>,
}

impl Shortcuts {
    pub fn path(workspace: &Path) -> PathBuf {
        workspace.with_file_name("shortcuts.json")
    }

    /// This never writes the file. A bad file is no reason to prevent startup.
    pub fn load(workspace: &Path) -> Self {
        let mut shortcuts = Self {
            stamp: Self::stamp(&Self::path(workspace)),
            path: Self::path(workspace),
            ..Self::default()
        };
        match std::fs::read(&shortcuts.path) {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(value) => {
                    shortcuts.overrides = Overrides::parse(&value);
                    if !shortcuts.overrides.diagnostics.is_empty() {
                        shortcuts.notice = Some(format!(
                            "{} shortcut problem(s) in {}; see Settings → Shortcuts.",
                            shortcuts.overrides.diagnostics.len(),
                            shortcuts.path.display()
                        ));
                    }
                }
                Err(error) => {
                    shortcuts.problem(format!("Invalid JSON: {error}; using default shortcuts."))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => shortcuts.problem(format!(
                "Unable to read shortcuts: {error}; using default shortcuts."
            )),
        }
        shortcuts
    }

    fn stamp(path: &Path) -> Option<(std::time::SystemTime, u64)> {
        let metadata = std::fs::metadata(path).ok()?;
        Some((metadata.modified().ok()?, metadata.len()))
    }

    /// Focus/Settings checks only stat the file; parse only when mtime or length changes.
    /// This never imports or writes. Missing files are a stable stamp too.
    pub fn reload(&mut self, workspace: &Path) -> bool {
        let path = Self::path(workspace);
        if self.path == path && self.stamp == Self::stamp(&path) {
            return false;
        }
        *self = Self::load(workspace);
        true
    }

    pub(super) fn problem(&mut self, message: String) {
        self.notice = Some(format!("{}: {message}", self.path.display()));
        self.overrides.diagnostics.push(Diagnostic {
            severity: "warning".into(),
            source: Some("settings".into()),
            message,
            ..Diagnostic::default()
        });
    }
}

/// One-time nivox import at Settings::load, before workspace saves discard the
/// unknown legacy field. The only writer: never overwrite an existing file.
pub(super) fn import_nivox(workspace: &Value, path: &Path) -> Result<Option<usize>> {
    if path.exists() {
        return Ok(None);
    }
    let Some(entries) = workspace.get("keybindings").and_then(Value::as_array) else {
        return Ok(None);
    };
    let defaults = defaults().bindings;
    let imported: Vec<_> = entries
        .iter()
        .cloned()
        .filter_map(|mut entry| {
            if let Some(object) = entry.as_object_mut() {
                if let Some(keys) = object.remove("key") {
                    object.insert("keys".into(), keys);
                }
                let keys = object
                    .get("keys")
                    .and_then(Value::as_str)
                    .and_then(|k| normalize(k).ok());
                match object.get("action").and_then(Value::as_str) {
                    Some("client.commands") => {
                        object.insert("action".into(), "client.palette.toggle".into());
                    }
                    Some("pane.close") if keys.as_deref() == Some(&["ctrl+shift+w".into()]) => {
                        object.insert("action".into(), "client.close".into());
                    }
                    _ => {}
                }
                if defaults.iter().any(|b| {
                    Some(&b.keys) == keys.as_ref()
                        && object.get("action").and_then(Value::as_str) == Some(b.action.as_str())
                        && object.get("args") == b.args.as_ref()
                }) {
                    return None;
                }
            }
            Some(entry)
        })
        .collect();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(file, &imported)?;
    Ok(Some(imported.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{keymap, settings::Settings};
    use serde_json::json;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "rex-shortcuts-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn workspace(&self) -> PathBuf {
            self.0.join("workspace.json")
        }
        fn shortcuts(&self) -> PathBuf {
            self.0.join("shortcuts.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn reload_is_metadata_gated_and_never_imports_or_writes() {
        let fixture = Fixture::new();
        let mut shortcuts = Shortcuts::load(&fixture.workspace());
        assert!(!shortcuts.reload(&fixture.workspace()));
        std::fs::write(fixture.shortcuts(), r#"[{"keys":"ctrl+k","action":null}]"#).unwrap();
        assert!(shortcuts.reload(&fixture.workspace()));
        assert!(!shortcuts.reload(&fixture.workspace()));
        assert_eq!(shortcuts.overrides.layer.removals.len(), 1);
        std::fs::write(fixture.shortcuts(), "bad").unwrap();
        assert!(shortcuts.reload(&fixture.workspace()));
        assert_eq!(shortcuts.overrides.diagnostics.len(), 1);
        assert!(!shortcuts.reload(&fixture.workspace()));
        std::fs::write(
            fixture.workspace(),
            r#"{"keybindings":[{"key":"ctrl+k","action":null}]}"#,
        )
        .unwrap();
        std::fs::remove_file(fixture.shortcuts()).unwrap();
        assert!(shortcuts.reload(&fixture.workspace()));
        assert!(shortcuts.overrides.layer.removals.is_empty());
        assert!(!fixture.shortcuts().exists());
        assert!(!shortcuts.reload(&fixture.workspace()));
    }

    #[test]
    fn missing_workspace_still_loads_sibling_shortcuts() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.shortcuts(),
            r#"[{"keys":"ctrl+tab","action":null}]"#,
        )
        .unwrap();
        let settings = Settings::load(&fixture.workspace()).unwrap();
        assert_eq!(settings.shortcut_file.path, fixture.shortcuts());
        assert_eq!(settings.shortcut_file.overrides.layer.removals.len(), 1);
        assert!(settings.shortcut_file.notice.is_none());
        assert!(!fixture.workspace().exists());
    }

    #[test]
    fn missing_shortcuts_are_not_created_and_workspace_never_embeds_them() {
        let fixture = Fixture::new();
        let settings = Settings::load(&fixture.workspace()).unwrap();
        settings.save(&fixture.workspace()).unwrap();
        assert!(!fixture.shortcuts().exists());
        let saved: Value =
            serde_json::from_slice(&std::fs::read(fixture.workspace()).unwrap()).unwrap();
        assert!(saved.get("shortcuts").is_none());
        assert!(saved.get("shortcut_file").is_none());
        assert!(settings.shortcut_file.overrides.diagnostics.is_empty());
        assert!(settings.shortcut_file.notice.is_none());
    }

    #[test]
    fn invalid_json_starts_with_defaults_and_keeps_a_problem_after_notice_is_taken() {
        let fixture = Fixture::new();
        std::fs::write(fixture.shortcuts(), "[ broken").unwrap();
        let mut settings = Settings::load(&fixture.workspace()).unwrap();
        assert!(
            settings
                .shortcut_file
                .notice
                .take()
                .unwrap()
                .contains("using default shortcuts")
        );
        assert!(settings.shortcut_file.notice.is_none());
        assert_eq!(settings.shortcut_file.overrides.diagnostics.len(), 1);
        assert_eq!(
            settings.shortcut_file.overrides.diagnostics[0]
                .source
                .as_deref(),
            Some("settings")
        );
        let map = keymap::resolve(&[defaults()], &settings.shortcut_file.overrides, &[]);
        assert_eq!(map.bindings.len(), defaults().bindings.len());
        settings.save(&fixture.workspace()).unwrap();
        assert_eq!(
            std::fs::read_to_string(fixture.shortcuts()).unwrap(),
            "[ broken"
        );
    }

    #[test]
    fn invalid_entry_and_hand_edits_survive_preference_saves_verbatim() {
        let fixture = Fixture::new();
        let original =
            r#"[ {"keys":"ctrl+b>","action":"pane.zoom"}, {"keys":"ctrl+tab","action":null} ]"#;
        std::fs::write(fixture.shortcuts(), original).unwrap();
        let mut settings = Settings::load(&fixture.workspace()).unwrap();
        assert_eq!(settings.shortcut_file.overrides.diagnostics.len(), 1);
        assert_eq!(settings.shortcut_file.overrides.layer.removals.len(), 1);
        settings.save(&fixture.workspace()).unwrap();
        assert_eq!(
            std::fs::read_to_string(fixture.shortcuts()).unwrap(),
            original
        );
        let edit = "[\n  {\"keys\":\"ctrl+k\", \"action\": null}\n]\n";
        std::fs::write(fixture.shortcuts(), edit).unwrap();
        settings.ui_scale = 1.2;
        settings.save(&fixture.workspace()).unwrap();
        assert_eq!(std::fs::read_to_string(fixture.shortcuts()).unwrap(), edit);
    }

    #[test]
    fn nivox_import_runs_at_load_once_and_drops_copied_defaults() {
        let fixture = Fixture::new();
        let legacy = json!({"keybindings":[
            {"key":"Control+Shift+P", "action":"client.commands"},
            {"key":"ctrl+shift+w", "action":"pane.close"},
            {"key":"ctrl+alt+d", "action":"pane.split", "args":{"direction":"down"}},
            {"key":"ctrl+b>c", "action":"client.tab.new"},
            {"key":"ctrl+k", "action":null},
            {"key":"ctrl+b>", "action":"pane.zoom"}
        ]});
        std::fs::write(fixture.workspace(), legacy.to_string()).unwrap();
        let first = Settings::load(&fixture.workspace()).unwrap();
        assert_eq!(first.shortcut_file.imported, Some(3));
        assert_eq!(
            first.shortcut_file.overrides.layer.bindings[0].keys,
            ["ctrl+b", "c"]
        );
        assert_eq!(first.shortcut_file.overrides.diagnostics.len(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(fixture.workspace()).unwrap()).unwrap(),
            legacy
        );
        let imported = std::fs::read(fixture.shortcuts()).unwrap();
        let second = Settings::load(&fixture.workspace()).unwrap();
        assert_eq!(second.shortcut_file.imported, None);
        assert_eq!(std::fs::read(fixture.shortcuts()).unwrap(), imported);
        second.save(&fixture.workspace()).unwrap();
        let saved: Value =
            serde_json::from_slice(&std::fs::read(fixture.workspace()).unwrap()).unwrap();
        assert!(saved.get("keybindings").is_none());
        assert_eq!(std::fs::read(fixture.shortcuts()).unwrap(), imported);
    }

    #[test]
    fn an_existing_even_invalid_override_file_is_never_imported_over() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.workspace(),
            r#"{"keybindings":[{"key":"ctrl+k","action":null}]}"#,
        )
        .unwrap();
        std::fs::write(fixture.shortcuts(), "bad json").unwrap();
        let settings = Settings::load(&fixture.workspace()).unwrap();
        assert_eq!(settings.shortcut_file.imported, None);
        assert!(settings.shortcut_file.notice.is_some());
        assert_eq!(
            std::fs::read_to_string(fixture.shortcuts()).unwrap(),
            "bad json"
        );
    }

    #[test]
    fn unreadable_shortcuts_fall_back_without_failing_workspace_load() {
        let fixture = Fixture::new();
        std::fs::create_dir(fixture.shortcuts()).unwrap();
        let settings = Settings::load(&fixture.workspace()).unwrap();
        assert!(
            settings
                .shortcut_file
                .notice
                .unwrap()
                .contains("Unable to read shortcuts")
        );
        assert!(settings.shortcut_file.overrides.layer.bindings.is_empty());
        assert_eq!(settings.shortcut_file.overrides.diagnostics.len(), 1);
    }
}
