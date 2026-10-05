//! Opt-in real filesystem watcher test. Temporarily edits a trusted UI label,
//! restores it with RAII, and never modifies the user's vault or the runtime.
mod support;
use makepad_test::{Selector, run_with_config};

struct Restore {
    path: std::path::PathBuf,
    content: String,
}
impl Drop for Restore {
    fn drop(&mut self) {
        std::fs::write(&self.path, &self.content).expect("restore UI source after hot reload test");
    }
}

#[test]
#[ignore = "temporarily edits ui.rs; run alone with --ignored after normal checks"]
fn script_style_reload_preserves_document_and_restores_original_style() {
    let root = support::library::root("hot_reload");
    let _ = std::fs::remove_dir_all(&root);
    let mut config = support::library::config("hot_reload", &root);
    config.app_args.push("--hot".into());
    run_with_config(config, |app| {
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let document = "# 热重载测试\n\n正文不应因 UI 样式修改而变化";
        support::library::fill_document(&app, document);
        app.locator(Selector::id("comment_input")).click();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui.rs");
        let original = std::fs::read_to_string(&path).unwrap();
        let restore = Restore {
            path,
            content: original,
        };
        let marker = "Label{text: \"DocGenie\" draw_text";
        assert_eq!(restore.content.matches(marker).count(), 1);
        let edited = restore
            .content
            .replace(marker, "Label{text: \"DocGenie HOT\" draw_text");
        let revision = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "revision_label")
            .unwrap()
            .text
            .unwrap();
        std::fs::write(&restore.path, edited).unwrap();
        app.locator(Selector::widget_type("Label").text_exact("DocGenie HOT"))
            .wait_visible();
        app.locator(Selector::id("live_editor"))
            .assert_text(document);
        app.locator(Selector::id("revision_label"))
            .assert_text(&revision);
        println!("hot-reloaded native UI: {}", app.screenshot().display());
        std::fs::write(&restore.path, &restore.content).unwrap();
        app.locator(Selector::widget_type("Label").text_exact("DocGenie"))
            .wait_visible();
        app.locator(Selector::id("live_editor"))
            .assert_text(document);
        app.locator(Selector::id("revision_label"))
            .assert_text(revision);
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(root);
}
