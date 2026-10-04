mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, run_with_config};
#[test]
fn local_comments_autosave_and_restore_without_external_requests() {
    let root = support::library::root("comments_restart");
    let _ = std::fs::remove_dir_all(&root);
    run_with_config(support::library::config("save_comments", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        support::library::fill_document(&app, "评论的原段落");
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("comment_input"))
            .fill("举一个例子");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n举一个例子");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
    })
    .unwrap();
    run_with_config(support::library::config("restore_comments", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("live_editor"))
            .wait_text("评论的原段落");
        app.locator(Selector::id("comments_label"))
            .wait_text("评论 (1)");
        app.locator(Selector::id("transcript"))
            .assert_text("你\n举一个例子");
        app.locator(Selector::id("agent_hint"))
            .assert_text("Agent 默认关闭 · 问 AI 可单条授权");
        // Auto-focus: switching to preview mode reveals the thread's
        // highlighted block without an explicit locate button.
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("live_editor")).wait_hidden();
        app.locator(Selector::id("badge")).wait_visible().click();
        app.locator(Selector::id("message_body"))
            .wait_text("举一个例子");
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
