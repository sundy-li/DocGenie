mod support;
use makepad_test::{KeyCode, Selector};

/// Separate key events are essential: pasting "## ABC" skips the empty-heading
/// parser state that used to eat a typed '#' on every native redraw.
#[test]
fn heading_prefix_shows_live_and_renders_on_blur() {
    support::library::run("heading_live", |app| {
        support::library::fill_document(&app, "# 标题\n\n");
        app.locator(Selector::id("active_line")).wait_visible();
        for (typed, expected) in [
            ("#", "#"),
            ("#", "##"),
            (" ", "## "),
            ("A", "## A"),
            ("B", "## AB"),
            ("C", "## ABC"),
        ] {
            app.type_text(typed);
            app.locator(Selector::id("active_line")).wait_text(expected);
            app.locator(Selector::id("live_editor"))
                .wait_text(format!("# 标题\n\n{expected}"));
        }
        println!("incremental heading active: {}", app.screenshot().display());
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("active_line")).wait_hidden();
        let snap = app.widget_snapshot();
        let rendered = snap
            .iter()
            .find(|w| w.id == "rendered" && w.visible)
            .unwrap();
        assert!(rendered.height > 30, "ABC must be a real H2, not body text");
        app.locator(Selector::id("live_editor"))
            .assert_text("# 标题\n\n## ABC");
        let screenshot = app.screenshot();
        let image = image::open(&screenshot).unwrap().to_rgb8();
        let window = snap.iter().find(|w| w.id == "main_window").unwrap();
        let scale = image.width() as f64 / window.width as f64;
        let blue = ((rendered.y as f64 * scale) as u32
            ..((rendered.y + rendered.height) as f64 * scale) as u32)
            .flat_map(|y| {
                ((rendered.x as f64 * scale) as u32
                    ..((rendered.x + rendered.width) as f64 * scale) as u32)
                    .map(move |x| (x, y))
            })
            .filter(|(x, y)| {
                let p = image.get_pixel(*x, *y).0;
                p[0] < 65 && p[1] > 45 && p[1] < 160 && p[2] > 135
            })
            .count();
        assert!(blue > 80, "blurred H2 must actually draw blue glyphs");
        println!("incremental heading blurred: {}", screenshot.display());
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line")).wait_text("## ABC");
        app.press_key(KeyCode::End);
        for expected in ["## AB", "## A", "## ", "##", "#"] {
            app.press_key(KeyCode::Backspace);
            app.locator(Selector::id("active_line")).wait_text(expected);
            app.locator(Selector::id("live_editor"))
                .wait_text(format!("# 标题\n\n{expected}"));
        }
        app.type_text("# 中文");
        app.locator(Selector::id("active_line"))
            .wait_text("## 中文");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("active_line"))
            .assert_text("## 中文");
    });
}
