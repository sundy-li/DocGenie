mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};
fn mods(shift: bool) -> KeyModifiers {
    KeyModifiers {
        logo: true,
        shift,
        ..Default::default()
    }
}
fn select_prefix(app: &makepad_test::TestApp, count: usize) {
    app.press_key(KeyCode::Home);
    for _ in 0..count {
        app.press_key_with_modifiers(
            KeyCode::ArrowRight,
            KeyModifiers {
                shift: true,
                ..Default::default()
            },
        );
    }
}
#[test]
fn partial_comment_quotes_anchors_and_highlights_only_selected_unicode() {
    support::library::run("precise_comment", |app| {
        let source = "# 评论\n\n选中词 后面仍然是正文，不能扩为整个段落。";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        select_prefix(&app, 3);
        app.press_key_with_modifiers(KeyCode::KeyM, mods(true));
        app.locator(Selector::id("composer_quote"))
            .wait_value("选中词");
        app.locator(Selector::id("composer_input"))
            .fill("仅解释这个词");
        app.locator(Selector::id("composer_submit")).click();
        app.locator(Selector::id("quote")).wait_text("选中词");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let root = support::library::root("precise_comment");
        let state = std::fs::read_dir(root.join(".state"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name() != "ui.json")
            .find_map(|e| {
                let data: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
                data.to_string().contains("仅解释这个词").then_some(data)
            })
            .unwrap();
        let thread = &state["workbench"]["threads"][0];
        assert_eq!(thread["original"], "选中词");
        assert_eq!(thread["messages"][0]["author"], "你");
        assert!(
            thread["messages"][0]["created_at"]
                .as_i64()
                .is_some_and(|t| t > 0)
        );
        app.locator(Selector::id("author").text_exact("你"))
            .wait_visible();
        let timestamp = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "message_time" && w.visible)
            .unwrap()
            .text
            .unwrap();
        assert!(timestamp.contains('月') && timestamp.contains(':'));
        assert_eq!(thread["range"]["start"], source.find("选中词").unwrap());
        assert_eq!(
            thread["range"]["end"],
            source.find("选中词").unwrap() + "选中词".len()
        );
        app.locator(Selector::id("comment_input")).click();
        let snap = app.widget_snapshot();
        let row = snap
            .iter()
            .find(|w| w.id == "rendered" && w.visible)
            .unwrap();
        let image = image::open(app.screenshot()).unwrap().to_rgb8();
        let window = snap.iter().find(|w| w.id == "main_window").unwrap();
        let scale = image.width() as f64 / window.width as f64;
        let yellow_x: Vec<_> =
            ((row.x as f64 * scale) as u32..((row.x + row.width) as f64 * scale) as u32)
                .filter(|x| {
                    ((row.y as f64 * scale) as u32..((row.y + row.height) as f64 * scale) as u32)
                        .any(|y| {
                            let p = image.get_pixel(*x, y).0;
                            p[0] > 240 && p[1] > 215 && p[2] < 230
                        })
                })
                .collect();
        assert!(
            yellow_x.len() > 15,
            "selected words need a visible comment highlight"
        );
        assert!(
            yellow_x.len() < 150,
            "must not highlight full row: {}",
            yellow_x.len()
        );
        app.locator(Selector::id("live_editor")).assert_text(source);
        println!("precise inline comment: {}", app.screenshot().display());
        app.locator(Selector::id("quote_jump")).click();
        app.locator(Selector::id("active_line")).wait_visible();
        app.locator(Selector::id("live_editor")).assert_text(source);
    });
}
#[test]
fn reading_partial_selection_uses_precise_source_range_not_paragraph() {
    support::library::run("reading_precise_comment", |app| {
        let source = "# 阅读\n\nAlpha Beta Gamma";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("markdown")).wait_visible();
        let snap = app.widget_snapshot();
        let row = snap
            .iter()
            .find(|w| w.id == "markdown" && w.visible)
            .unwrap();
        let x = row.x as f64 + 5.0;
        let y = row.y as f64 + 12.0;
        app.forward(vec![
            makepad_test::StudioToApp::MouseDown(makepad_test::RemoteMouseDown {
                button_raw_bits: makepad_test::MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 1.0,
                modifiers: Default::default(),
            }),
            makepad_test::StudioToApp::MouseMove(makepad_test::RemoteMouseMove {
                x: x + 50.0,
                y,
                time: 1.1,
                modifiers: Default::default(),
            }),
            makepad_test::StudioToApp::MouseUp(makepad_test::RemoteMouseUp {
                button_raw_bits: makepad_test::MouseButton::PRIMARY.bits(),
                x: x + 50.0,
                y,
                time: 1.2,
                modifiers: Default::default(),
            }),
        ]);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        let quote = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "composer_quote")
            .unwrap()
            .value
            .unwrap();
        assert!(
            !quote.is_empty() && quote.len() < "Alpha Beta Gamma".len(),
            "precise reading quote: {quote:?}"
        );
        assert!("Alpha Beta Gamma".contains(&quote));
        app.locator(Selector::id("composer_input"))
            .fill("仅选区评论");
        app.locator(Selector::id("composer_submit")).click();
        let actual = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .text
            .unwrap();
        assert_eq!(actual.trim(), quote);
        assert!(actual.len() < "Alpha Beta Gamma".len());
        app.locator(Selector::id("live_editor")).assert_text(source);
    });
}
#[test]
fn cmd_z_undoes_document_after_row_blur_and_format_and_keeps_drafts_local() {
    support::library::run("document_cmd_z", |app| {
        let source = "# 撤销\n\n正文";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.press_key(KeyCode::End);
        app.type_text("新增");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 撤销\n\n正文新增");
        app.locator(Selector::id("mode_edit")).click();
        app.press_key_with_modifiers(KeyCode::KeyZ, mods(false));
        app.locator(Selector::id("live_editor")).wait_text(source);
        app.locator(Selector::id("mode_read")).click();
        app.locator(Selector::id("rendered")).click();
        app.press_key_with_modifiers(KeyCode::KeyA, mods(false));
        app.locator(Selector::id("fmt_bold")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 撤销\n\n**正文**");
        app.locator(Selector::id("rendered")).click();
        app.press_key_with_modifiers(KeyCode::KeyZ, mods(false));
        app.locator(Selector::id("live_editor")).wait_text(source);
        app.locator(Selector::id("active_line")).wait_visible();
        app.press_key(KeyCode::End);
        app.type_text("甲");
        app.type_text("乙");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 撤销\n\n正文甲乙");
        app.press_key_with_modifiers(KeyCode::KeyZ, mods(false));
        app.locator(Selector::id("live_editor"))
            .wait_text("# 撤销\n\n正文甲");
        app.press_key_with_modifiers(KeyCode::KeyZ, mods(false));
        app.locator(Selector::id("live_editor")).wait_text(source);
        app.locator(Selector::id("comment_input")).click();
        app.type_text("评论草稿");
        app.press_key_with_modifiers(KeyCode::KeyZ, mods(false));
        app.locator(Selector::id("live_editor")).assert_text(source);
    });
}
