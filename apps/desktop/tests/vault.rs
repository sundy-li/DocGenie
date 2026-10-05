mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, run_with_config};

fn heading(name: &str) -> Selector {
    Selector::id("heading").text_exact(name)
}

#[test]
fn tree_folders_rename_tabs_switcher_and_right_tab_persist() {
    let root = support::library::root("vault_tree");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("项目")).unwrap();
    std::fs::write(root.join("项目/计划.md"), "# 计划\n\n内容\n").unwrap();
    run_with_config(support::library::config("vault_tree_a", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        // The active document's ancestors are expanded on load.
        app.locator(heading("项目")).wait_visible();
        app.locator(heading("计划")).wait_visible();
        app.locator(Selector::id("tab0")).wait_visible();

        app.locator(heading("项目")).click();
        app.locator(heading("计划")).wait_hidden();
        app.locator(heading("项目")).click();
        app.locator(heading("计划")).wait_visible();

        app.locator(Selector::id("new_folder")).click();
        app.locator(Selector::id("dialog_overlay")).wait_visible();
        app.locator(Selector::id("dialog_input")).fill("资料");
        app.locator(Selector::id("dialog_ok")).click();
        app.locator(Selector::id("dialog_overlay")).wait_hidden();
        assert!(root.join("项目/资料").is_dir());

        app.locator(heading("计划")).click();
        app.locator(Selector::id("rename_node")).click();
        app.locator(Selector::id("dialog_input")).fill("路线图");
        app.locator(Selector::id("dialog_ok")).click();
        app.locator(Selector::id("dialog_overlay")).wait_hidden();
        app.locator(heading("路线图")).wait_visible();
        assert!(root.join("项目/路线图.md").is_file());
        assert!(!root.join("项目/计划.md").exists());
        app.locator(Selector::id("title").text_exact("路线图"))
            .wait_visible();

        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("tab1")).wait_visible();
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        assert!(root.join("项目/未命名文档.md").is_file());

        app.press_key_with_modifiers(
            KeyCode::KeyO,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        app.locator(Selector::id("switcher_overlay")).wait_visible();
        app.locator(Selector::id("switcher_input")).fill("路线");
        app.press_key(KeyCode::ReturnKey);
        app.locator(Selector::id("switcher_overlay")).wait_hidden();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 计划\n\n内容\n");

        app.locator(Selector::id("tab_outline")).click();
        app.locator(Selector::id("outline_body")).wait_visible();
        app.locator(Selector::id("comments_body")).wait_hidden();
        app.locator(Selector::id("tab_comments")).click();
        app.locator(Selector::id("comments_body")).wait_visible();
        app.locator(Selector::id("tab_outline")).click();
        println!("vault layout: {}", app.screenshot().display());
    })
    .unwrap();
    run_with_config(support::library::config("vault_tree_b", &root), |app| {
        support::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("outline_body")).wait_visible();
        app.locator(Selector::id("tab1")).wait_visible();
        app.locator(heading("资料")).wait_visible();

        app.locator(heading("未命名文档")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("delete_node")).click();
        app.locator(Selector::id("dialog_ok")).click();
        app.locator(Selector::id("dialog_overlay")).wait_hidden();
        assert!(!root.join("项目/未命名文档.md").exists());
        app.locator(Selector::id("tab1")).wait_hidden();
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn editing_the_title_renames_the_file() {
    support::library::run("title_renames_file", |app| {
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        app.locator(Selector::id("title_input")).fill("改过的标题");
        app.locator(heading("改过的标题")).wait_visible();
        app.locator(Selector::id("title").text_exact("改过的标题"))
            .wait_visible();
    });
}
