mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestApp};

fn comment(app: &TestApp) {
    support::wait_for_ink(app, "revision_label");
    support::library::fill_document(app, "原始段落");
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..KeyModifiers::default()
        },
    );
    support::library::right_click_editor(app);
    app.locator(Selector::id("quote")).wait_text("原始段落");
    app.locator(Selector::id("comment_input"))
        .fill("补充一个常用例子");
    app.locator(Selector::id("comment_send")).click();
    app.locator(Selector::id("transcript"))
        .wait_text("你\n补充一个常用例子");
}
#[test]
fn local_comment_and_reply_do_not_edit_without_authorization() {
    support::library::run(
        "local_comment_and_reply_do_not_edit_without_authorization",
        |app| {
            comment(&app);
            app.locator(Selector::id("comment_input"))
                .fill("再讲清楚实现边界");
            app.locator(Selector::id("comment_send")).click();
            app.locator(Selector::id("transcript"))
                .wait_text("你\n补充一个常用例子\n\n你\n再讲清楚实现边界");
            app.locator(Selector::id("comments_label"))
                .wait_text("评论 (1)");
            // Without enabling default_auto_modify in Preferences, the Agent
            // never runs on a sent comment, so the editor stays unchanged.
            app.locator(Selector::id("live_editor"))
                .assert_text("原始段落");
            // Sanity check: the only place Agent auto-modify lives now is
            // Preferences; without enabling it, the document stays intact.
            support::open_settings(&app);
            app.locator(Selector::id("preferences_overlay"))
                .wait_visible();
            app.locator(Selector::id("preferences_done")).click();
            app.locator(Selector::id("preferences_overlay"))
                .wait_hidden();
            app.locator(Selector::id("live_editor"))
                .assert_text("原始段落");
            println!(
                "comment UI: {}",
                support::wait_for_ink(&app, "comments_label").display()
            );
        },
    );
}
#[test]
fn default_reading_outline_renders_and_navigates() {
    support::library::run("default_reading_outline_renders_and_navigates", |app| {
        support::library::fill_document(&app, "# 文档\n\n## 章节\n\n正文");
        app.locator(Selector::id("mode_edit")).click();
        support::wait_for_ink(&app, "preview");
        app.locator(Selector::id("live_editor")).wait_hidden();
        println!("reading layout: {}", app.screenshot().display());
        app.locator(Selector::id("tab_outline")).click();
        let row = app.locator(Selector::id("heading").text_exact("  章节"));
        row.wait_visible().click();
        app.locator(Selector::id("preview_panel")).wait_visible();
        app.locator(Selector::id("live_panel")).wait_hidden();
    });
}

#[test]
fn resolved_comment_stays_local_and_rejects_new_reply() {
    support::library::run(
        "resolved_comment_stays_local_and_rejects_new_reply",
        |app| {
            comment(&app);
            app.locator(Selector::id("thread_resolve_toggle")).click();
            app.locator(Selector::id("thread_state"))
                .wait_text("已解决 · 不再自动处理");
            app.locator(Selector::id("comment_input")).fill("不应提交");
            app.locator(Selector::id("comment_send")).click();
            app.locator(Selector::id("transcript"))
                .assert_text("你\n补充一个常用例子");
            app.locator(Selector::id("live_editor"))
                .assert_text("原始段落");
        },
    );
}

#[test]
fn reading_selection_supports_right_click_comment() {
    support::library::run("reading_context", |app| {
        support::library::fill_document(&app, "唯一段落原文");
        app.locator(Selector::id("mode_edit")).click();
        support::wait_for_ink(&app, "preview");
        app.locator(Selector::id("markdown")).click();
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click(&app, "markdown");
        app.locator(Selector::id("quote")).wait_text("唯一段落原文");
        app.locator(Selector::id("comment_input"))
            .fill("阅读模式批注");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n阅读模式批注");
    });
}
#[test]
fn title_renders_once_and_mode_icons_toggle() {
    support::library::run("title_once", |app| {
        support::library::fill_document(&app, "# 标题\n\n正文");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("title_input")).wait_value("标题");
        app.locator(Selector::id("rendered").nth(0))
            .wait_text("正文");
        let widgets = app.widget_snapshot();
        assert_eq!(widgets.iter().filter(|w| w.id == "rendered").count(), 1);
        app.locator(Selector::id("mode_read")).wait_hidden();
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("mode_read")).wait_visible();
        app.locator(Selector::id("preview")).wait_visible();
        let widgets = app.widget_snapshot();
        assert_eq!(widgets.iter().filter(|w| w.id == "markdown").count(), 1);
        app.locator(Selector::id("mode_read")).click();
        app.locator(Selector::id("live_editor")).wait_visible();
        println!("icons: {}", app.screenshot().display());
    });
}
