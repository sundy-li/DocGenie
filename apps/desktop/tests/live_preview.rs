mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};
use std::time::{Duration, Instant};

#[test]
fn active_line_source_other_lines_markdown_and_blur() {
    support::library::run("live_preview", |app| {
        support::library::fill_document(&app, "# 标题\n\n正文 **粗体**\n下一行\n");
        app.locator(Selector::id("rendered").nth(0))
            .wait_visible()
            .click();
        app.locator(Selector::id("active_line"))
            .wait_visible()
            .wait_value("正文 **粗体**");
        app.locator(Selector::id("title_input")).wait_value("标题");
        app.press_key(KeyCode::End);
        app.type_text("新");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 标题\n\n正文 **粗体**新\n下一行\n");
        app.press_key(KeyCode::ArrowDown);
        app.locator(Selector::id("active_line"))
            .wait_value("下一行");
        app.press_key(KeyCode::ArrowUp);
        app.locator(Selector::id("active_line"))
            .wait_value("正文 **粗体**新");
        let widgets = app.widget_snapshot();
        assert!(
            widgets
                .iter()
                .any(|w| w.id == "rendered" && w.text.as_deref() == Some("下一行"))
        );
        assert!(
            !widgets
                .iter()
                .any(|w| w.id == "rendered" && w.text.as_deref() == Some("# 标题"))
        );
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("active_line")).wait_hidden();
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("live_panel")).wait_hidden();
        app.locator(Selector::id("preview"))
            .wait_text("# 标题\n\n正文 **粗体**新\n下一行\n");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        println!("live-preview UI: {}", app.screenshot().display());
    });
}

#[test]
fn newline_navigation_caret_blinks_and_comment_targets_source() {
    support::library::run("live_preview_caret", |app| {
        support::library::fill_document(&app, "第一行\n第二行\n");
        app.locator(Selector::id("rendered").nth(0)).click();
        app.locator(Selector::id("active_line"))
            .wait_visible()
            .fill("修改第一行");
        app.press_key(KeyCode::End);
        app.press_key(KeyCode::ReturnKey);
        app.locator(Selector::id("live_editor"))
            .wait_text("修改第一行\n\n第二行\n");
        app.locator(Selector::id("active_line")).wait_value("");
        app.type_text("新行");
        app.locator(Selector::id("live_editor"))
            .wait_text("修改第一行\n新行\n第二行\n");
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        support::library::right_click(&app, "active_line");
        app.locator(Selector::id("quote")).wait_value("新行");
        app.locator(Selector::id("comment_input"))
            .fill("这行的批注");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n这行的批注");
        // The active Live Preview line itself must visibly blink.
        app.locator(Selector::id("rendered").nth(0)).click();
        app.locator(Selector::id("active_line"))
            .wait_visible()
            .click();
        app.press_key(KeyCode::ArrowRight);
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut low = usize::MAX;
        let mut high = 0;
        while Instant::now() < deadline {
            let image = image::open(app.screenshot()).unwrap().to_rgb8();
            let count = image
                .pixels()
                .filter(|p| p[2] > 180 && p[0] < 80 && p[1] > 60 && p[1] < 160)
                .count();
            low = low.min(count);
            high = high.max(count);
            if high > low + 8 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            high > low + 8,
            "caret must alternate rendered visible/hidden: low={low}, high={high}"
        );
    });
}
