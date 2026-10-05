mod support;
use makepad_test::{Selector, run_with_config};

#[test]
fn saved_size_and_code_fonts_restore_and_document_views_scale_without_edits() {
    let root = support::library::root("preferences_restore");
    std::fs::create_dir_all(&root).unwrap();
    let prefs = root.join("preferences.json");
    std::fs::write(&prefs, r#"{"default_auto_modify":false,"editor_font_size":20,"auto_save_enabled":true,"editor_code_fonts":["Definitely Not Installed DocGenie Font"]}"#).unwrap();
    run_with_config(
        support::library::config("preferences_restore", &root),
        |app| {
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
            support::library::fill_document(&app, "# 设置\n\n正文 `code` 中文");
            app.locator(Selector::id("comment_input")).click();
            let before = app
                .widget_snapshot()
                .into_iter()
                .find(|w| w.id == "rendered")
                .unwrap();
            let revision = app
                .widget_snapshot()
                .into_iter()
                .find(|w| w.id == "revision_label")
                .unwrap()
                .text
                .unwrap();
            support::open_settings(&app);
            app.locator(Selector::id("pref_font_size")).wait_value("20");
            app.locator(Selector::id("pref_font_status"))
                .wait_text("当前代码字体: 内置 Liberation Mono");
            app.locator(Selector::id("font_name").nth(0))
                .wait_text("Definitely Not Installed DocGenie Font");
            app.locator(Selector::id("pref_font_size")).fill("28");
            app.locator(Selector::id("preferences_done")).click();
            let after = app
                .widget_snapshot()
                .into_iter()
                .find(|w| w.id == "rendered")
                .unwrap();
            assert!(
                after.height > before.height,
                "rendered document must really scale"
            );
            app.locator(Selector::id("rendered")).click();
            let active = app
                .widget_snapshot()
                .into_iter()
                .find(|w| w.id == "active_line")
                .unwrap();
            assert_eq!(
                active.height, after.height,
                "activation must preserve configured font metrics"
            );
            app.locator(Selector::id("mode_edit")).click();
            app.locator(Selector::id("preview_panel")).wait_visible();
            let reading = app
                .widget_snapshot()
                .into_iter()
                .find(|w| w.id == "markdown")
                .unwrap();
            assert!(
                reading.height >= before.height,
                "reading must use configured size too"
            );
            app.locator(Selector::id("revision_label"))
                .assert_text(revision);
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
        },
    )
    .unwrap();
    run_with_config(
        support::library::config("preferences_restart", &root),
        |app| {
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
            support::open_settings(&app);
            app.locator(Selector::id("pref_font_size")).wait_value("28");
            app.locator(Selector::id("pref_font_status"))
                .wait_text("当前代码字体: 内置 Liberation Mono");
            app.locator(Selector::id("preferences_done")).click();
            app.locator(Selector::id("live_editor"))
                .assert_text("# 设置\n\n正文 `code` 中文");
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
