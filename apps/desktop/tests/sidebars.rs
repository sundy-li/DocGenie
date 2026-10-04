mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestApp};
use std::time::{Duration, Instant};

fn width(app: &TestApp, id: &str) -> f32 {
    app.widget_snapshot()
        .iter()
        .find(|w| w.id == id)
        .unwrap()
        .width as f32
}

fn wait_width(app: &TestApp, id: &str, expected: f32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let actual = width(app, id);
        if (actual - expected).abs() < 2.0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{id}: expected width {expected}, got {actual}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn independent_sidebars_reclaim_space_and_preserve_drafts() {
    support::library::run("independent_sidebars", |app| {
        let text = "# 文档\n\n侧栏收起时文档内容保持不变。";
        support::library::fill_document(&app, text);
        app.locator(Selector::id("comment_input"))
            .fill("尚未发送的评论草稿");
        let original = width(&app, "live_panel");

        app.locator(Selector::id("collapse_navigation")).click();
        app.locator(Selector::id("navigation_panel")).wait_hidden();
        app.locator(Selector::id("expand_navigation"))
            .wait_visible();
        app.locator(Selector::id("comments_panel")).wait_visible();
        wait_width(&app, "live_panel", (original + 249.0).min(760.0));

        app.locator(Selector::id("collapse_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_hidden();
        app.locator(Selector::id("expand_comments")).wait_visible();
        wait_width(&app, "live_panel", (original + 590.0).min(760.0));
        app.locator(Selector::id("live_editor")).assert_text(text);
        println!(
            "collapsed sidebars: {}",
            support::wait_for_ink(&app, "revision_label").display()
        );

        app.locator(Selector::id("expand_navigation")).click();
        app.locator(Selector::id("navigation_panel")).wait_visible();
        app.locator(Selector::id("comments_panel")).wait_hidden();
        wait_width(&app, "live_panel", (original + 341.0).min(760.0));
        app.locator(Selector::id("expand_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_visible();
        wait_width(&app, "live_panel", original);
        app.locator(Selector::id("comment_input"))
            .assert_value("尚未发送的评论草稿");

        app.locator(Selector::id("mode_edit")).click();
        support::wait_for_ink(&app, "preview");
        let preview_width = width(&app, "preview_panel");
        app.locator(Selector::id("collapse_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_hidden();
        wait_width(&app, "preview_panel", (preview_width + 341.0).min(760.0));
        app.locator(Selector::id("expand_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_visible();
        wait_width(&app, "preview_panel", preview_width);
    });
}

#[test]
fn right_click_comment_reopens_hidden_comments_only() {
    support::library::run("comment_reopens_sidebar", |app| {
        support::library::fill_document(&app, "待批注的原始段落");
        app.locator(Selector::id("collapse_navigation")).click();
        app.locator(Selector::id("collapse_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_hidden();
        app.locator(Selector::id("rendered").nth(0)).click();
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("comments_panel")).wait_visible();
        app.locator(Selector::id("navigation_panel")).wait_hidden();
        app.locator(Selector::id("expand_comments")).wait_hidden();
        app.locator(Selector::id("quote"))
            .wait_text("待批注的原始段落");
        app.locator(Selector::id("comment_input"))
            .fill("请补充说明");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n请补充说明");
        app.locator(Selector::id("collapse_comments")).click();
        app.locator(Selector::id("comments_panel")).wait_hidden();
        app.locator(Selector::id("expand_comments")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n请补充说明");
    });
}
