use project_store::{StoreError, load, save};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "agent-docs-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("document.md")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn roundtrip_and_atomic_replacement() {
    let dir = Temp::new();
    let path = dir.file();
    assert_eq!(load(&path).unwrap(), None);
    save(&path, "# 中文\n\n👋", None).unwrap();
    assert_eq!(load(&path).unwrap().as_deref(), Some("# 中文\n\n👋"));
    save(&path, "新内容", Some("# 中文\n\n👋")).unwrap();
    assert_eq!(load(&path).unwrap().as_deref(), Some("新内容"));
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
}
#[test]
fn new_path_never_overwrites_existing_file() {
    let dir = Temp::new();
    let path = dir.file();
    fs::write(&path, "external").unwrap();
    assert!(matches!(
        save(&path, "mine", None),
        Err(StoreError::ExternalChange)
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), "external");
}
#[test]
fn external_edit_is_preserved() {
    let dir = Temp::new();
    let path = dir.file();
    save(&path, "baseline", None).unwrap();
    fs::write(&path, "external").unwrap();
    assert!(matches!(
        save(&path, "mine", Some("baseline")),
        Err(StoreError::ExternalChange)
    ));
    assert_eq!(load(&path).unwrap().as_deref(), Some("external"));
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
}
#[test]
fn malformed_and_large_files_are_rejected() {
    let dir = Temp::new();
    let path = dir.file();
    fs::write(&path, [0xff]).unwrap();
    assert!(matches!(load(&path), Err(StoreError::InvalidUtf8)));
    fs::write(&path, vec![b'x'; document_core::MAX_DOCUMENT_BYTES + 1]).unwrap();
    assert!(matches!(load(&path), Err(StoreError::TooLarge)));
    assert!(matches!(
        save(
            &path,
            &"x".repeat(document_core::MAX_DOCUMENT_BYTES + 1),
            None
        ),
        Err(StoreError::TooLarge)
    ));
}
#[test]
fn invalid_paths_and_missing_parent_fail() {
    assert!(matches!(
        load(std::path::Path::new("relative.md")),
        Err(StoreError::InvalidPath)
    ));
    let dir = Temp::new();
    assert!(matches!(
        load(&dir.0.join("file.exe")),
        Err(StoreError::InvalidPath)
    ));
    assert!(matches!(
        save(&dir.0.join("missing/doc.md"), "abc", None),
        Err(StoreError::Io(_))
    ));
}
#[test]
fn cooperating_lock_blocks_writer() {
    let dir = Temp::new();
    let path = dir.file();
    fs::write(dir.0.join(".document.md.agent-docs.lock"), "").unwrap();
    assert!(matches!(save(&path, "abc", None), Err(StoreError::Busy)));
    assert!(!path.exists());
}
#[cfg(unix)]
#[test]
fn symlink_target_is_not_followed() {
    let dir = Temp::new();
    let path = dir.file();
    let target = dir.0.join("target.md");
    fs::write(&target, "private").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(matches!(load(&path), Err(StoreError::UnsafeTarget)));
    assert!(matches!(
        save(&path, "abc", Some("private")),
        Err(StoreError::UnsafeTarget)
    ));
    assert_eq!(fs::read_to_string(target).unwrap(), "private");
}
