mod support;
use makepad_test::{Selector, run_with_config};
#[test]
fn create_autosave_switch_and_restart_plain_markdown() {
    let root = support::library::root("autosave_restart");
    let _ = std::fs::remove_dir_all(&root);
    let text = "# 我的文档\n\n普通 Markdown，修改即自动保存。";
    run_with_config(support::library::config("autosave_create", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        support::library::fill_document(&app, text).wait_text(text);
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let files: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|s| s == "md"))
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(std::fs::read_to_string(files[0].path()).unwrap(), text);
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        assert_eq!(
            std::fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|s| s == "md"))
                .count(),
            2
        );
    })
    .unwrap();
    run_with_config(support::library::config("autosave_restart", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "open_button" || w.id == "file_path")
        );
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_edit_blocks_autosave_and_document_switch() {
    support::library::run("external_autosave", |app| {
        let root = support::library::root("external_autosave");
        let path = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.path().extension().is_some_and(|s| s == "md"))
            .unwrap()
            .path();
        std::fs::write(&path, "磁盘外部内容").unwrap();
        support::library::fill_document(&app, "不能覆盖外部内容");
        app.locator(Selector::id("save_label"))
            .wait_text("自动保存失败 · 保留内存内容");
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .assert_text("不能覆盖外部内容");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "磁盘外部内容");
    });
}
