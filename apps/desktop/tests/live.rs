//! Explicit opt-in only: sends synthetic text to configured MiniMax, not user files.
mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, run_with_config};
use std::time::Duration;

#[test]
#[ignore = "real MiniMax calls with synthetic text; may incur token charges"]
fn minimax_two_round_native_comment_workflow() {
    let root = support::library::root("live_minimax");
    let _ = std::fs::remove_dir_all(&root);
    let mut config = support::library::config("live_minimax", &root);
    config.action_timeout = Duration::from_secs(75);
    run_with_config(config, |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        support::library::fill_document(
            &app,
            "# 自动编辑验收\n\n这是一个文档编辑工具。\n\n## 结尾\n\n这段必须保持不变。",
        );
        // Select all for this synthetic test: model receives synthetic text only.
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("comment_input"))
            .fill("只把第二段换成：这是一款本地 Markdown 编辑工具。保留标题和结尾。");
        // Enable auto-modify via Preferences (the only place this setting
        // lives now; the sidebar checkbox is gone).
        support::open_settings(&app);
        app.locator(Selector::id("pref_nav_agent")).click();
        app.locator(Selector::id("pref_default_auto")).click();
        app.locator(Selector::id("preferences_done")).click();
        app.locator(Selector::id("preferences_overlay"))
            .wait_hidden();
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("status_label"))
            .wait_text("已修改段落并回复 · 可撤销 · 自动保存");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("comment_input"))
            .fill("继续只修改第二段，换成：本地 Markdown 编辑，评论驱动完善。其他内容不动。");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("status_label"))
            .wait_text("已修改段落并回复 · 可撤销 · 自动保存");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let widgets = app.widget_snapshot();
        let text = widgets
            .iter()
            .find(|w| w.id == "live_editor")
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert!(text.contains("本地 Markdown 编辑，评论驱动完善。"));
        assert!(text.contains("这段必须保持不变。"));
        let transcript = widgets
            .iter()
            .find(|w| w.id == "transcript")
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert_eq!(transcript.matches("\n\nAgent\n").count(), 2);
        let md = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.path().extension().is_some_and(|s| s == "md"))
            .unwrap();
        assert_eq!(std::fs::read_to_string(md.path()).unwrap(), text);
        // Auto-focus replaces the explicit locate button.
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("badge")).wait_visible();
        println!("live MiniMax UI screenshot: {}", app.screenshot().display());
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
