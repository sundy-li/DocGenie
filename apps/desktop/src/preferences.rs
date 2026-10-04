//! Persistent, app-level settings kept outside the project store.
//!
//! Lives at `$AGENT_DOCS_HOME/preferences.json` when the test harness sets a
//! scratch directory, otherwise at `~/Library/Application Support/agent-docs/`
//! on macOS. Missing or unreadable files fall back to defaults rather than
//! failing to launch.
//!
//! Scope discipline (mirrors AGENTS.md):
//! - No API keys are read or written here. Keys come from env / OS credential
//!   store; the Preferences UI only displays endpoint and model provenance.
//! - Preferences are app-level, not per-project; reopening a project must not
//!   silently rewrite the file.
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const MAX_BYTES: usize = 16 * 1024;
const FILE_NAME: &str = "preferences.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    #[serde(default = "default_auto_modify")]
    pub default_auto_modify: bool,
    #[serde(default = "default_font_size")]
    pub editor_font_size: u32,
    #[serde(default = "default_auto_save")]
    pub auto_save_enabled: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            default_auto_modify: default_auto_modify(),
            editor_font_size: default_font_size(),
            auto_save_enabled: default_auto_save(),
        }
    }
}

const fn default_auto_modify() -> bool {
    false
}
const fn default_font_size() -> u32 {
    14
}
const fn default_auto_save() -> bool {
    true
}

/// Resolves the on-disk path. `AGENT_DOCS_HOME` wins (used by the test harness
/// to keep scratch directories isolated). On other hosts the macOS-standard
/// `~/Library/Application Support/agent-docs/` is used; we are macOS-first per
/// `docs/implementation.md`.
pub fn preferences_path() -> PathBuf {
    if let Ok(root) = env::var("AGENT_DOCS_HOME") {
        return PathBuf::from(root).join(FILE_NAME);
    }
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("agent-docs")
        .join(FILE_NAME)
}

pub fn load() -> Preferences {
    let path = preferences_path();
    match fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<Preferences>(&raw) {
            Ok(parsed) if raw.len() <= MAX_BYTES => parsed,
            Ok(_) => {
                eprintln!(
                    "agent-docs: preferences file exceeded {} bytes, ignoring",
                    MAX_BYTES
                );
                Preferences::default()
            }
            Err(error) => {
                eprintln!(
                    "agent-docs: ignoring malformed preferences at {}: {error}",
                    path.display()
                );
                Preferences::default()
            }
        },
        Err(_) => Preferences::default(),
    }
}

/// Atomic write: write to a sibling temp file then rename. The caller controls
/// where the file lives via `preferences_path()`; this function only guards
/// against partial writes on crash. Returns the path actually written so the
/// caller can surface it in the status bar.
pub fn save(preferences: &Preferences) -> Result<PathBuf, std::io::Error> {
    let target = preferences_path();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(preferences)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    if serialized.len() > MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "preferences exceed size limit",
        ));
    }
    let temp = temp_sibling(&target);
    fs::write(&temp, serialized.as_bytes())?;
    fs::rename(&temp, &target)?;
    Ok(target)
}

fn temp_sibling(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "preferences.json".into());
    name.push(".tmp");
    target.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_scratch_home(suffix: &str, body: impl FnOnce(&Path)) {
        let dir = std::env::temp_dir().join(format!(
            "agent-docs-prefs-{}-{}-{}",
            std::process::id(),
            suffix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // AGENT_DOCS_HOME wins over HOME so we never have to mutate HOME itself.
        // Tests serialise via ENV_LOCK and cargo's --test-threads=1 default.
        unsafe {
            env::set_var("AGENT_DOCS_HOME", &dir);
        }
        body(&dir);
        unsafe {
            env::remove_var("AGENT_DOCS_HOME");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn defaults_when_file_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        with_scratch_home("missing", |_| {
            let prefs = load();
            assert!(!prefs.default_auto_modify);
            assert_eq!(prefs.editor_font_size, 14);
            assert!(prefs.auto_save_enabled);
        });
    }

    #[test]
    fn round_trip_preserves_all_fields() {
        let _guard = ENV_LOCK.lock().unwrap();
        with_scratch_home("roundtrip", |_| {
            let saved = Preferences {
                default_auto_modify: true,
                editor_font_size: 18,
                auto_save_enabled: false,
            };
            let path = save(&saved).unwrap();
            assert!(path.ends_with("preferences.json"));
            let loaded = load();
            assert_eq!(loaded, saved);
        });
    }

    #[test]
    fn malformed_file_falls_back_to_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        with_scratch_home("malformed", |_| {
            let path = preferences_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"{ this is not json").unwrap();
            let loaded = load();
            assert_eq!(loaded, Preferences::default());
        });
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let _guard = ENV_LOCK.lock().unwrap();
        with_scratch_home("unknown", |_| {
            let path = preferences_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                br#"{"default_auto_modify": false, "editor_font_size": 14,
                     "auto_save_enabled": true, "secret": "leak"}"#,
            )
            .unwrap();
            let loaded = load();
            // deny_unknown_fields rejects the whole payload, not just the field.
            assert_eq!(loaded, Preferences::default());
        });
    }

    #[test]
    fn save_is_atomic_via_rename() {
        let _guard = ENV_LOCK.lock().unwrap();
        with_scratch_home("atomic", |_| {
            let saved = Preferences {
                default_auto_modify: false,
                editor_font_size: 20,
                auto_save_enabled: true,
            };
            save(&saved).unwrap();
            let target = preferences_path();
            let mut temp_name = target.file_name().unwrap().to_owned();
            temp_name.push(".tmp");
            let temp = target.with_file_name(temp_name);
            assert!(!temp.exists(), "temp file should be renamed away");
        });
    }
}
