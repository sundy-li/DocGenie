//! Preferences overlay (⌘,): Obsidian-style nav pages, code-font priority
//! list, and instant-apply semantics — every control persists and applies
//! the moment it changes, with no save/cancel step.
mod support;
use makepad_test::Selector;

fn preferences_json(root: &std::path::Path) -> serde_json::Value {
    let path = root.join("preferences.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("preferences.json written"))
        .expect("preferences.json is valid json")
}

#[test]
fn settings_panel_nav_font_list_and_live_apply() {
    let root = support::library::root("settings_panel_nav_font_list_and_live_apply");
    support::library::run("settings_panel_nav_font_list_and_live_apply", |app| {
        support::library::fill_document(&app, "# 文档\n\n正文 `code` 段落");
        support::open_settings(&app);
        app.locator(Selector::id("preferences_overlay"))
            .wait_visible();
        // The 外观 page is the default; it carries the font controls.
        app.locator(Selector::id("pref_page_appearance"))
            .wait_visible();
        app.locator(Selector::id("font_name").nth(0))
            .wait_text("Source Code Pro");
        app.locator(Selector::id("font_name").nth(1))
            .wait_text("Bangla Sangam MN");
        app.locator(Selector::id("font_name").nth(2))
            .wait_text("Andale Mono");
        println!("panel: {}", app.screenshot().display());
        // Instant apply: editing the font size persists immediately,
        // without any save button.
        app.locator(Selector::id("pref_font_size")).fill("18");
        let saved = preferences_json(&root);
        assert_eq!(saved["editor_font_size"], 18);
        assert_eq!(
            saved["editor_code_fonts"]
                .as_array()
                .expect("code font list persisted")
                .len(),
            3
        );
        // The first resolvable family applies live (Source Code Pro is
        // not installed on macOS; Bangla Sangam MN is).
        app.locator(Selector::id("pref_font_status"))
            .wait_text("当前代码字体: Bangla Sangam MN");
        // Removing the top priority row shifts the list up and persists.
        app.locator(Selector::id("font_remove").nth(0)).click();
        app.locator(Selector::id("font_name").nth(0))
            .wait_text("Bangla Sangam MN");
        app.locator(Selector::id("font_name").nth(1))
            .wait_text("Andale Mono");
        let saved = preferences_json(&root);
        assert_eq!(
            saved["editor_code_fonts"],
            serde_json::json!(["Bangla Sangam MN", "Andale Mono"])
        );
        // Nav switches pages; the agent page carries the auto-modify toggle.
        app.locator(Selector::id("pref_nav_agent")).click();
        app.locator(Selector::id("pref_page_agent")).wait_visible();
        app.locator(Selector::id("pref_default_auto"))
            .wait_visible();
        app.locator(Selector::id("pref_nav_editor")).click();
        app.locator(Selector::id("pref_auto_save")).wait_visible();
        app.locator(Selector::id("pref_nav_appearance")).click();
        app.locator(Selector::id("pref_font_size")).wait_visible();
        // 完成 closes the overlay; the document is untouched.
        app.locator(Selector::id("preferences_done")).click();
        app.locator(Selector::id("preferences_overlay"))
            .wait_hidden();
        println!("editor-18pt: {}", app.screenshot().display());
        app.locator(Selector::id("live_editor"))
            .assert_text("# 文档\n\n正文 `code` 段落");
    });
}
