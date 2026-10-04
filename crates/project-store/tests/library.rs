use document_core::Workbench;
use project_store::{StoreError, library::Library};
use std::{fs, path::PathBuf};
use uuid::Uuid;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("agent-docs-library-{}", Uuid::new_v4())))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn ordinary_markdown_and_comments_roundtrip() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    let mut loaded = lib.create().unwrap();
    loaded.workbench.edit("# 测试\n\n正文").unwrap();
    loaded.workbench.comment(10..16, "补充").unwrap();
    lib.save(
        &loaded.document,
        &loaded.workbench.snapshot(),
        Some(&loaded.baseline),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(&loaded.document.path).unwrap(),
        "# 测试\n\n正文"
    );
    let restored = lib.load(&loaded.document).unwrap();
    assert!(restored.recovered_comments);
    assert_eq!(restored.workbench.thread_count(), 1);
    assert_eq!(lib.list().unwrap()[0].name, "测试");
}
#[test]
fn external_change_refuses_overwrite_and_invalidates_stale_metadata() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    let loaded = lib.create().unwrap();
    fs::write(&loaded.document.path, "# 外部修改").unwrap();
    let w = Workbench::new("App content").unwrap();
    assert!(matches!(
        lib.save(&loaded.document, &w.snapshot(), Some(&loaded.baseline)),
        Err(StoreError::ExternalChange)
    ));
    let restored = lib.load(&loaded.document).unwrap();
    assert!(!restored.recovered_comments);
    assert_eq!(restored.workbench.text(), "# 外部修改");
}
#[test]
fn new_documents_are_distinct_and_path_escape_is_rejected() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    let first = lib.create().unwrap();
    let second = lib.create().unwrap();
    assert_ne!(first.document.id, second.document.id);
    assert_eq!(lib.list().unwrap().len(), 2);
    let mut invalid = first.document;
    invalid.path = dir.0.join("../escape.md");
    assert!(matches!(lib.load(&invalid), Err(StoreError::InvalidPath)));
}
#[test]
fn vault_tree_has_folders_first_and_hides_internal_files() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    lib.create_folder("", "项目").unwrap();
    lib.create_note("项目", "计划").unwrap();
    lib.create_note("", "日记").unwrap();
    fs::write(dir.0.join("notes.txt"), "x").unwrap();
    let tree = lib.tree().unwrap();
    let view: Vec<_> = tree
        .iter()
        .map(|n| (n.rel.as_str(), n.name.as_str(), n.is_dir, n.depth))
        .collect();
    assert_eq!(
        view,
        [
            ("项目", "项目", true, 0),
            ("项目/计划.md", "计划", false, 1),
            ("日记.md", "日记", false, 0),
        ]
    );
}
#[test]
fn rename_keeps_comments_and_unique_names() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    lib.create_folder("", "a").unwrap();
    let mut note = lib.create_note("a", "想法").unwrap();
    assert_eq!(
        lib.create_note("a", "想法").unwrap().document.rel,
        "a/想法 1.md"
    );
    note.workbench.edit("# 想法\n\n正文").unwrap();
    note.workbench.comment(10..16, "评论").unwrap();
    lib.save(
        &note.document,
        &note.workbench.snapshot(),
        Some(&note.baseline),
    )
    .unwrap();
    let renamed = lib.rename("a", "b").unwrap();
    assert_eq!(renamed, "b");
    let doc = lib.document("b/想法.md").unwrap();
    assert_eq!(lib.load(&doc).unwrap().workbench.thread_count(), 1);
    assert!(lib.document("a/想法.md").is_err());
    let file = lib.rename("b/想法.md", "新名").unwrap();
    assert_eq!(file, "b/新名.md");
    assert_eq!(
        lib.load(&lib.document(&file).unwrap())
            .unwrap()
            .workbench
            .thread_count(),
        1
    );
    assert!(lib.rename("b/新名.md", "想法 1").is_err());
}
#[test]
fn unsafe_names_and_paths_are_rejected_and_delete_is_scoped() {
    let dir = Temp::new();
    let outside = Temp::new();
    fs::create_dir_all(&outside.0).unwrap();
    fs::write(outside.0.join("x.md"), "# x").unwrap();
    let lib = Library::open(dir.0.clone()).unwrap();
    for bad in ["../x", "a/b", ".hidden", "", "a:b"] {
        assert!(lib.create_note("", bad).is_err(), "{bad}");
        assert!(lib.create_folder("", bad).is_err(), "{bad}");
    }
    assert!(lib.create_note("../", "x").is_err());
    assert!(lib.document("../x.md").is_err());
    assert!(lib.delete("../x.md").is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside.0, dir.0.join("link")).unwrap();
        assert!(lib.create_note("link", "x").is_err());
        assert!(lib.delete("link").is_err());
        assert!(lib.tree().unwrap().is_empty());
    }
    lib.create_folder("", "d").unwrap();
    let keep = lib.create_note("d", "n").unwrap();
    fs::write(dir.0.join("d/other.txt"), "x").unwrap();
    assert!(lib.delete("d").is_err());
    fs::remove_file(dir.0.join("d/other.txt")).unwrap();
    lib.delete("d").unwrap();
    assert!(!keep.document.path.exists());
    assert!(outside.0.join("x.md").exists());
}

#[test]
fn file_name_follows_title_without_touching_legacy_or_untitled() {
    let root = std::env::temp_dir().join(format!("agent-docs-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let library = Library::open(root.clone()).unwrap();
    let note = library.create_note("", "旧名").unwrap();
    assert_eq!(
        library
            .sync_name(&note.document.rel, "# 新标题/一\n")
            .unwrap(),
        Some("新标题-一.md".into())
    );
    assert!(root.join("新标题-一.md").is_file());
    assert_eq!(
        library.sync_name("新标题-一.md", "# 新标题-一\n").unwrap(),
        None
    );
    assert_eq!(
        library.sync_name("新标题-一.md", "没有标题\n").unwrap(),
        None
    );
    let other = library.create_note("", "占用").unwrap();
    let renamed = library
        .sync_name(&other.document.rel, "# 新标题-一\n")
        .unwrap();
    assert_eq!(renamed.as_deref(), Some("新标题-一 1.md"));
    assert_eq!(
        library
            .sync_name("新标题-一 1.md", "# 新标题-一\n")
            .unwrap(),
        None
    );
    let legacy = library.create().unwrap();
    assert_eq!(
        library.sync_name(&legacy.document.rel, "# 别的\n").unwrap(),
        None
    );
    let _ = std::fs::remove_dir_all(root);
}

fn image_header() -> Vec<u8> {
    let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
    data.extend_from_slice(&13_u32.to_be_bytes());
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&2_u32.to_be_bytes());
    data.extend_from_slice(&1_u32.to_be_bytes());
    data
}
#[test]
fn attachments_are_bounded_immutable_and_survive_rename_and_undo() {
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    let mut note = lib.create_note("", "note").unwrap();
    let png = image_header();
    let rel = lib.store_image(&png).unwrap();
    assert_eq!(lib.read_image(&rel).unwrap(), png);
    assert_ne!(lib.store_image(&png).unwrap(), rel);
    note.workbench.edit(format!("![图片]({rel})")).unwrap();
    lib.save(
        &note.document,
        &note.workbench.snapshot(),
        Some(&note.baseline),
    )
    .unwrap();
    let new = lib.rename(&note.document.rel, "renamed").unwrap();
    let mut restored = lib.load(&lib.document(&new).unwrap()).unwrap();
    restored.workbench.undo().unwrap();
    assert!(!restored.workbench.text().contains(&rel));
    assert_eq!(lib.read_image(&rel).unwrap(), png);
    assert!(
        !lib.tree()
            .unwrap()
            .iter()
            .any(|n| n.rel.starts_with("assets"))
    );
    assert!(lib.delete("assets").is_err());
    assert!(lib.create_folder("", "assets").is_err());
    for path in [
        "../bad.png",
        "/tmp/bad.png",
        "assets/../bad.png",
        "assets/.hidden.png",
        "https://a/x.png",
    ] {
        assert!(lib.read_image(path).is_err());
    }
    assert!(lib.store_image(b"not png").is_err());
    let mut oversized = png;
    oversized[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
    oversized[20..24].copy_from_slice(&100_000_u32.to_be_bytes());
    assert!(lib.store_image(&oversized).is_err());
}
#[cfg(unix)]
#[test]
fn attachment_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let dir = Temp::new();
    let lib = Library::open(dir.0.clone()).unwrap();
    let png = image_header();
    let rel = lib.store_image(&png).unwrap();
    let path = dir.0.join(&rel);
    fs::remove_file(&path).unwrap();
    symlink(dir.0.join("target"), &path).unwrap();
    assert!(lib.read_image(&rel).is_err());
    fs::remove_dir_all(dir.0.join("assets")).unwrap();
    fs::create_dir(dir.0.join("other")).unwrap();
    symlink(dir.0.join("other"), dir.0.join("assets")).unwrap();
    assert!(lib.store_image(&png).is_err());
    assert!(lib.read_image(&rel).is_err());
}
