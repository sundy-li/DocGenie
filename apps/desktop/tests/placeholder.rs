mod support;
use makepad_test::Selector;

/// A fresh document opens with the caret in a white empty body row.
#[test]
fn fresh_doc_active_row_without_placeholder() {
    support::library::run("fresh_doc_visual", |app| {
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        app.locator(Selector::id("active_line")).wait_visible();
        std::thread::sleep(std::time::Duration::from_millis(700));
        let snap = app.widget_snapshot();
        let row = snap.iter().find(|w| w.id == "active_line").unwrap();
        assert!(
            row.text.as_deref().unwrap_or("").is_empty(),
            "empty row carries no text"
        );
        // Editing does not add a different block background.
        let shot = app.screenshot();
        let pixels = image::open(&shot).unwrap().to_rgb8();
        let root = snap.iter().find(|w| w.id == "main_window").unwrap();
        let scale = pixels.width() as f64 / root.width as f64;
        let (x0, y0) = (row.x as f64 * scale, row.y as f64 * scale);
        let (w, h) = (row.width as f64 * scale, row.height as f64 * scale);
        let mut white = 0;
        for y in (y0 as u32)..((y0 + h) as u32).min(pixels.height()) {
            for x in (x0 as u32)..((x0 + w) as u32).min(pixels.width()) {
                let p = pixels.get_pixel(x, y).0;
                if p.iter().all(|c| *c > 248) {
                    white += 1;
                }
            }
        }
        println!("fresh doc row: {}", shot.display());
        assert!(white > 500, "active row must stay white, got {white}");
    });
}
