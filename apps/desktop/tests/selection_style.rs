mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};

#[test]
fn full_selection_paints_existing_blocks_without_relayout_or_duplicate_title() {
    support::library::run("selection_style", |app| {
        let source = "# 文档标题\n\n第一段正文 English words\n\n## 二级标题\n\n- 第一项\n- 第二项\n\n| 列一 | 列二 |\n| --- | --- |\n| 单元 | 2 |\n\n第三段正文 **粗体**";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered").nth(0)).click();
        app.locator(Selector::id("active_line"))
            .wait_text("第一段正文 English words");
        let before = app.widget_snapshot();
        let revision = before
            .iter()
            .find(|w| w.id == "revision_label")
            .unwrap()
            .text
            .clone()
            .unwrap();
        let mods = KeyModifiers {
            logo: true,
            ..Default::default()
        };
        app.press_key_with_modifiers(KeyCode::KeyA, mods);
        app.press_key_with_modifiers(KeyCode::KeyA, mods);
        app.locator(Selector::id("active_line"))
            .assert_text("第一段正文 English words");
        let after = app.widget_snapshot();
        for old in before
            .iter()
            .filter(|w| w.id == "rendered" || w.id == "active_line" || w.id == "title_input")
        {
            let new = after
                .iter()
                .find(|w| w.id == old.id && w.text == old.text)
                .expect("all existing blocks must remain");
            assert_eq!(
                (new.x, new.y, new.width, new.height),
                (old.x, old.y, old.width, old.height),
                "full selection must not re-layout {}",
                old.id
            );
        }
        assert_eq!(after.iter().filter(|w| w.id == "active_line").count(), 1);
        app.locator(Selector::id("revision_label"))
            .assert_text(revision);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let screenshot = app.screenshot();
        let image = image::open(&screenshot).unwrap().to_rgb8();
        let window = after.iter().find(|w| w.id == "main_window").unwrap();
        let editor = after.iter().find(|w| w.id == "live_editor").unwrap();
        let scale = image.width() as f64 / window.width as f64;
        let (mut mint, mut ink) = (0, 0);
        for y in
            (editor.y as f64 * scale) as u32..((editor.y + editor.height) as f64 * scale) as u32
        {
            for x in
                (editor.x as f64 * scale) as u32..((editor.x + editor.width) as f64 * scale) as u32
            {
                let p = image.get_pixel(x, y).0;
                if p[0].abs_diff(225) <= 3 && p[1].abs_diff(239) <= 3 && p[2].abs_diff(237) <= 3 {
                    mint += 1;
                }
                if p[0] < 110 && p[1] < 145 && p[2] < 185 {
                    ink += 1;
                }
            }
        }
        assert!(mint > 3000, "must paint real mint bands: {mint}");
        assert!(ink > 1000, "bands must not cover glyphs: {ink}");
        for block in after.iter().filter(|w| w.id == "rendered") {
            let mut selected_pixels = 0;
            for y in
                (block.y as f64 * scale) as u32..((block.y + block.height) as f64 * scale) as u32
            {
                for x in
                    (block.x as f64 * scale) as u32..((block.x + block.width) as f64 * scale) as u32
                {
                    let p = image.get_pixel(x, y).0;
                    if p[0].abs_diff(225) <= 3 && p[1].abs_diff(239) <= 3 && p[2].abs_diff(237) <= 3
                    {
                        selected_pixels += 1;
                    }
                }
            }
            assert!(
                selected_pixels > 100,
                "every rendered block, including lists, needs a band: {:?}",
                block.text
            );
        }
        println!("unchanged full-selection layout: {}", screenshot.display());
        app.locator(Selector::id("add_comment")).click();
        app.locator(Selector::id("quote")).wait_text(source);
    });
}
