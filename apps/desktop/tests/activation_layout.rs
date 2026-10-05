mod support;
use makepad_test::Selector;

fn dark_ink(app: &makepad_test::TestApp, id: &str, index: usize) -> (u32, u32, u32) {
    // Visibility/tree replies may precede the next presented GPU frame.
    std::thread::sleep(std::time::Duration::from_millis(150));
    let snapshot = app.widget_snapshot();
    let image = image::open(app.screenshot()).unwrap().to_rgb8();
    let mut widgets: Vec<_> = snapshot.iter().filter(|w| w.id == id).collect();
    widgets.sort_by_key(|w| w.y);
    let widget = widgets[index];
    let window = snapshot.iter().find(|w| w.id == "main_window").unwrap();
    let scale = image.width() as f64 / window.width as f64;
    let mut bounds = (u32::MAX, u32::MAX, 0);
    for y in (widget.y as f64 * scale) as u32..((widget.y + widget.height) as f64 * scale) as u32 {
        for x in (widget.x as f64 * scale) as u32..((widget.x + widget.width) as f64 * scale) as u32
        {
            let rgb = image.get_pixel(x, y).0;
            if (rgb[0] < 100 && rgb[1] < 135 && rgb[2] < 180)
                && !(rgb[0] > rgb[1] && rgb[1] > rgb[2])
            {
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(y);
            }
        }
    }
    bounds
}

fn verify(source: &str, case: &str) {
    support::library::run(case, |app| {
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        let initial = app.widget_snapshot();
        let count = initial.iter().filter(|w| w.id == "rendered").count();
        let revision = initial
            .iter()
            .find(|w| w.id == "revision_label")
            .unwrap()
            .text
            .clone();
        for pass in 0..3 {
            for index in 0..count - 1 {
                let before = app.widget_snapshot();
                let mut rendered: Vec<_> = before.iter().filter(|w| w.id == "rendered").collect();
                rendered.sort_by_key(|w| w.y);
                let current = rendered[index];
                let following = rendered[index + 1];
                let old = (
                    current.x,
                    current.y,
                    current.width,
                    current.height,
                    following.y,
                );
                let old_ink = dark_ink(&app, "rendered", index);
                app.locator(Selector::id("rendered").nth(index)).click();
                app.locator(Selector::id("active_line")).wait_visible();
                let after = app.widget_snapshot();
                let active = after.iter().find(|w| w.id == "active_line").unwrap();
                let next = after
                    .iter()
                    .find(|w| w.id == "rendered" && w.text == following.text)
                    .unwrap();
                assert_eq!(
                    (active.x, active.y, active.width, active.height),
                    (old.0, old.1, old.2, old.3),
                    "active box must not move/resize {case} pass {pass} block {index}"
                );
                assert_eq!(
                    next.y, old.4,
                    "activation moves following text {case} pass {pass} block {index}"
                );
                if old_ink.0 != u32::MAX {
                    let new_ink = dark_ink(&app, "active_line", 0);
                    assert_eq!(
                        (new_ink.1, new_ink.2),
                        (old_ink.1, old_ink.2),
                        "visible baseline moves {case} pass {pass} block {index}"
                    );
                    // A focused heading intentionally exposes hashes; AA can
                    // change its leftmost dark pixel by one device pixel.
                    assert!(
                        new_ink.0.abs_diff(old_ink.0) <= 1,
                        "horizontal ink moves {case} pass {pass} block {index}: {old_ink:?} -> {new_ink:?}"
                    );
                }
                if pass == 0 {
                    println!("{case} block {index}: {}", app.screenshot().display());
                }
                app.locator(Selector::id("comment_input")).click();
                app.locator(Selector::id("active_line")).wait_hidden();
                let blurred = app.widget_snapshot();
                let next = blurred
                    .iter()
                    .find(|w| w.id == "rendered" && w.text == following.text)
                    .unwrap();
                assert_eq!(
                    next.y, old.4,
                    "blur moves following text {case} pass {pass} block {index}"
                );
                assert_eq!(
                    blurred
                        .iter()
                        .find(|w| w.id == "revision_label")
                        .unwrap()
                        .text,
                    revision,
                    "focus must not mutate Markdown"
                );
            }
        }
    });
}
#[test]
fn activation_preserves_body_headings_and_lists() {
    verify(
        "# 标题\n\n普通正文 English 中文\n\n## 二级标题\n\n- 第一项\n- 第二项\n\n3. first\n4. second\n\n末尾定位段落",
        "activation_layout",
    );
}
#[test]
fn activation_preserves_wrapped_text() {
    let paragraph = "Long English words need native word wrapping alongside 中文文字。 ".repeat(4);
    verify(
        &format!("# 标题\n\n{paragraph}\n\n末尾定位段落"),
        "activation_wrap",
    );
}
#[test]
fn activation_preserves_wrapped_list() {
    let paragraph = "Long English words need native word wrapping alongside 中文文字。 ".repeat(3);
    verify(
        &format!("# 标题\n\n- {paragraph}\n- 第二项\n\n末尾定位段落"),
        "activation_list_wrap",
    );
}
#[test]
fn activation_preserves_inline_styles() {
    verify(
        "# 标题\n\n普通 **粗体** *italic* 文本\n\n末尾定位段落",
        "activation_styles",
    );
}
#[test]
fn activation_preserves_wrapped_inline_styles() {
    let text = "普通正文 English words **粗体** *italic* 继续编辑。 ".repeat(6);
    verify(
        &format!("# 标题\n\n{text}\n\n末尾定位段落"),
        "activation_styles_wrap",
    );
}
