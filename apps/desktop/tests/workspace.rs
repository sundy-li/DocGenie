mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestApp, run_with_config};

fn rect(app: &TestApp, id: &str) -> (f64, f64) {
    let widgets = app.widget_snapshot();
    let w = widgets.iter().find(|w| w.id == id).unwrap();
    (w.x as f64, w.width as f64)
}
fn select_body(app: &TestApp) {
    app.locator(Selector::id("comment_input")).click();
    app.locator(Selector::id("rendered")).wait_visible().click();
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..Default::default()
        },
    );
    support::library::right_click_editor(app);
}
#[test]
fn article_is_bounded_and_centered_and_search_reuses_switcher() {
    support::library::run("article_workspace", |app| {
        support::library::fill_document(
            &app,
            "# 写作工作台\n\n正文优先，保留标签和本地文档。\n\n## 章节\n\n中文段落。",
        );
        app.locator(Selector::id("comment_input")).click();
        for nav in [true, false] {
            if !nav {
                app.locator(Selector::id("toggle_navigation")).click();
            }
            for comments in [true, false] {
                if !comments {
                    app.locator(Selector::id("toggle_comments")).click();
                }
                let (surface_x, surface_width) = rect(&app, "article_surface");
                let (column_x, column_width) = rect(&app, "article_column");
                let expected = (surface_width - 48.0).min(760.0);
                assert!(
                    (column_width - expected).abs() <= 2.0,
                    "expected {expected}, got {column_width}"
                );
                assert!((column_x - surface_x - (surface_width - column_width) / 2.0).abs() <= 2.0);
                if !comments {
                    app.locator(Selector::id("toggle_comments")).click();
                }
            }
        }
        app.locator(Selector::id("toggle_navigation")).click();
        app.locator(Selector::id("document_search")).click();
        app.locator(Selector::id("switcher_overlay")).wait_visible();
        app.press_key(KeyCode::Escape);
        app.locator(Selector::id("switcher_overlay")).wait_hidden();
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("tab_outline")).click();
        app.locator(Selector::id("heading").text_exact("  章节"))
            .click();
        app.locator(Selector::id("preview_panel")).wait_visible();
        app.locator(Selector::id("live_panel")).wait_hidden();
        println!(
            "workspace reading: {}",
            support::wait_for_ink(&app, "preview").display()
        );
    });
}
#[test]
fn narrow_workspace_keeps_document_visible_and_draft_when_navigation_opens() {
    let root = support::library::root("narrow_workspace");
    let _ = std::fs::remove_dir_all(&root);
    let mut config = support::library::config("narrow_workspace", &root);
    config.app_args.push("--window-size=900x800".into());
    run_with_config(config, |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("navigation_panel")).wait_hidden();
        app.locator(Selector::id("comments_panel")).wait_visible();
        app.locator(Selector::id("comment_input"))
            .fill("窄窗口保留草稿");
        assert!(rect(&app, "article_column").1 >= 460.0);
        app.locator(Selector::id("toggle_comments")).click();
        app.locator(Selector::id("navigation_panel")).wait_visible();
        app.locator(Selector::id("toggle_comments")).click();
        app.locator(Selector::id("navigation_panel")).wait_hidden();
        app.locator(Selector::id("comment_input"))
            .assert_value("窄窗口保留草稿");
        println!(
            "narrow workspace: {}",
            support::wait_for_ink(&app, "revision_label").display()
        );
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn thread_overview_filters_reopens_and_preserves_per_thread_drafts() {
    support::library::run("thread_overview", |app| {
        support::library::fill_document(&app, "同一段落上的多个评论");
        select_body(&app);
        app.locator(Selector::id("comment_input"))
            .fill("第一条评论");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("filter_open"))
            .wait_text("未解决 1");
        app.locator(Selector::id("comment_input"))
            .fill("第一条的草稿");
        app.locator(Selector::id("rendered")).wait_visible();
        select_body(&app);
        app.locator(Selector::id("comment_input"))
            .fill("第二条评论");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("filter_open"))
            .wait_text("未解决 2");
        app.locator(Selector::id("comment_input"))
            .fill("第二条的草稿");
        app.locator(Selector::id("thread_open").nth(0)).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n第一条评论");
        app.locator(Selector::id("comment_input"))
            .assert_value("第一条的草稿");
        // Blur the revealed source row so the following selection remains predictable.
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("thread_open").nth(1)).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n第二条评论");
        app.locator(Selector::id("comment_input"))
            .assert_value("第二条的草稿");
        app.locator(Selector::id("thread_resolve_toggle")).click();
        app.locator(Selector::id("filter_resolved"))
            .wait_text("已解决 1");
        app.locator(Selector::id("filter_open"))
            .wait_text("未解决 1");
        app.locator(Selector::id("filter_resolved")).click();
        app.locator(Selector::id("thread_open")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n第二条评论");
        app.locator(Selector::id("thread_resolve_toggle"))
            .wait_text("重开");
        app.locator(Selector::id("thread_resolve_toggle")).click();
        app.locator(Selector::id("filter_resolved"))
            .wait_text("已解决 0");
        app.locator(Selector::id("threads_empty"))
            .wait_text("还没有已解决的评论。");
        app.locator(Selector::id("filter_open")).click();
        app.locator(Selector::id("filter_open"))
            .wait_text("未解决 2");
        app.locator(Selector::id("live_editor"))
            .assert_text("同一段落上的多个评论");
        println!(
            "thread overview: {}",
            support::wait_for_ink(&app, "comments_label").display()
        );
    });
}
