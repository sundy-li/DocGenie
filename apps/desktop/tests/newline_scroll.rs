mod support;
use makepad_test::{KeyCode, Selector};

#[test]
fn repeated_enter_keeps_caret_visible_and_shows_document_scrollbar() {
    support::library::run("newline_scroll", |app| {
        support::library::fill_document(&app, "# 连续换行\n\n起点");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.press_key(KeyCode::End);
        let mut source = "# 连续换行\n\n起点".to_owned();
        for i in 0..38 {
            app.press_key(KeyCode::ReturnKey);
            source.push('\n');
            app.locator(Selector::id("live_editor")).wait_text(&source);
            let snapshot = app.widget_snapshot();
            let viewport = snapshot.iter().find(|w| w.id == "live_editor").unwrap();
            let active = snapshot
                .iter()
                .find(|w| w.id == "active_line" && w.visible)
                .expect("Enter must create a visible focused row");
            assert!(
                active.y >= viewport.y && active.y + active.height <= viewport.y + viewport.height,
                "Enter {i}: active row must stay inside viewport: {active:?}, viewport={viewport:?}"
            );
        }
        app.type_text("末尾仍能输入");
        source.push_str("末尾仍能输入");
        app.locator(Selector::id("live_editor")).wait_text(&source);
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let screenshot = app.screenshot();
        let image = image::open(&screenshot).unwrap().to_rgb8();
        let snapshot = app.widget_snapshot();
        let window = snapshot.iter().find(|w| w.id == "main_window").unwrap();
        let viewport = snapshot.iter().find(|w| w.id == "live_editor").unwrap();
        let scale = image.width() as f64 / window.width as f64;
        let mut bar_pixels = 0;
        for x in ((viewport.x + viewport.width - 12) as f64 * scale) as u32
            ..((viewport.x + viewport.width) as f64 * scale) as u32
        {
            for y in (viewport.y as f64 * scale) as u32
                ..((viewport.y + viewport.height) as f64 * scale) as u32
            {
                let p = image.get_pixel(x, y).0;
                if p[0].abs_diff(150) < 35 && p[1].abs_diff(155) < 35 && p[2].abs_diff(165) < 35 {
                    bar_pixels += 1;
                }
            }
        }
        assert!(
            bar_pixels > 100,
            "overflowing document must show visible scrollbar: {bar_pixels}"
        );
        println!("continuous Enter with scrollbar: {}", screenshot.display());
        app.press_key(KeyCode::ReturnKey);
        source.push('\n');
        app.type_text("保存后继续");
        source.push_str("保存后继续");
        app.locator(Selector::id("live_editor")).wait_text(source);
    });
}
