mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};
fn select_all(app: &makepad_test::TestApp) {
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..Default::default()
        },
    );
}
#[test]
fn active_body_keeps_bold_and_link_labels_without_source() {
    support::library::run("styled_body", |app| {
        support::library::fill_document(
            &app,
            "# 测试\n\n普通 **粗体** [链接](https://example.com) 正文",
        );
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line"))
            .wait_text("普通 粗体 链接 正文");
        app.press_key(KeyCode::End);
        app.type_text("新增");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 测试\n\n普通 **粗体** [链接](https://example.com) 正文新增");
        println!("active styled body: {}", app.screenshot().display());
        app.press_key_with_modifiers(
            KeyCode::KeyZ,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("live_editor"))
            .wait_text("# 测试\n\n普通 **粗体** [链接](https://example.com) 正文");
    });
}
#[test]
fn active_heading_shows_hashes_blue_and_blur_hides_them() {
    support::library::run("styled_heading", |app| {
        support::library::fill_document(&app, "# 文档\n\n## 蓝色标题\n\n正文");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered").nth(0)).click();
        app.locator(Selector::id("active_line"))
            .wait_text("## 蓝色标题");
        let screenshot = app.screenshot();
        let image = image::open(&screenshot).unwrap().to_rgb8();
        let blue = image
            .pixels()
            .filter(|p| p[0] < 65 && p[1] > 45 && p[1] < 160 && p[2] > 135)
            .count();
        assert!(
            blue > 80,
            "heading must really render blue, got {blue} pixels"
        );
        println!("active blue heading: {}", screenshot.display());
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("active_line")).wait_hidden();
        app.locator(Selector::id("title_input")).click();
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "title_prefix" && w.visible)
        );
        app.locator(Selector::id("title_input")).wait_value("文档");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("title_input")).wait_value("文档");
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "title_prefix" && w.visible)
        );
    });
}
#[test]
fn link_label_can_be_edited_and_comment_uses_exact_label() {
    support::library::run("styled_link", |app| {
        let source = "# 链接\n\n[链接文字](https://example.com/path)";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::widget_type("DocMarkdownLink"))
            .click();
        app.locator(Selector::id("active_line"))
            .wait_text("链接文字");
        select_all(&app);
        app.type_text("新链接");
        let updated = "# 链接\n\n[新链接](https://example.com/path)";
        app.locator(Selector::id("live_editor")).wait_text(updated);
        select_all(&app);
        support::library::right_click_editor(&app);
        app.locator(Selector::id("quote")).wait_text("新链接");
        app.locator(Selector::id("comment_input"))
            .fill("保持链接地址");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n保持链接地址");
        app.locator(Selector::id("live_editor"))
            .assert_text(updated);
    });
}
#[test]
fn table_cell_edits_preserve_pipes_alignment_and_neighbor_cells() {
    support::library::run("styled_table", |app| {
        let source = "# 表格\n\n| 名称 | 数量 |\n| :--- | ---: |\n| 中文 | 2 |";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::widget_type("StyledInput").text_exact("中文"))
            .click();
        select_all(&app);
        app.type_text("新内容");
        app.locator(Selector::id("live_editor"))
            .wait_text(source.replace("中文", "新内容"));
        println!("editable table: {}", app.screenshot().display());
        app.press_key_with_modifiers(
            KeyCode::KeyZ,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("live_editor")).wait_text(source);
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("preview_panel")).wait_visible();
    });
}
