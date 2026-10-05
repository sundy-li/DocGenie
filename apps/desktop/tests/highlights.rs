mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};

fn select_block(app: &makepad_test::TestApp, index: usize) {
    let widgets = app.widget_snapshot();
    let mut blocks: Vec<_> = widgets.iter().filter(|w| w.id == "markdown").collect();
    blocks.sort_by_key(|w| w.y);
    let block = blocks[index];
    let x = block.x as f64 + 15.0;
    let y = block.y as f64 + (block.height as f64 / 2.0).min(20.0);
    app.forward(vec![
        makepad_test::StudioToApp::MouseDown(makepad_test::RemoteMouseDown {
            button_raw_bits: makepad_test::MouseButton::PRIMARY.bits(),
            x,
            y,
            time: 0.0,
            modifiers: Default::default(),
        }),
        makepad_test::StudioToApp::MouseUp(makepad_test::RemoteMouseUp {
            button_raw_bits: makepad_test::MouseButton::PRIMARY.bits(),
            x,
            y,
            time: 0.1,
            modifiers: Default::default(),
        }),
    ]);
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..KeyModifiers::default()
        },
    );
    app.forward(vec![
        makepad_test::StudioToApp::MouseDown(makepad_test::RemoteMouseDown {
            button_raw_bits: makepad_test::MouseButton::SECONDARY.bits(),
            x: block.x as f64 + 30.0,
            y: block.y as f64 + (block.height as f64 / 2.0).min(20.0),
            time: 0.0,
            modifiers: Default::default(),
        }),
        makepad_test::StudioToApp::MouseUp(makepad_test::RemoteMouseUp {
            button_raw_bits: makepad_test::MouseButton::SECONDARY.bits(),
            x: block.x as f64 + 30.0,
            y: block.y as f64 + (block.height as f64 / 2.0).min(20.0),
            time: 0.1,
            modifiers: Default::default(),
        }),
    ]);
    app.locator(Selector::id("add_comment"))
        .wait_visible()
        .click();
}
#[test]
fn duplicate_formatted_paragraph_highlight_and_cards() {
    support::library::run("formatted_highlight", |app| {
        support::library::fill_document(&app, "# 文档\n\n相同 **段落**\n\n相同 **段落**\n");
        app.locator(Selector::id("mode_edit")).click();
        support::wait_for_ink(&app, "preview");
        select_block(&app, 1);
        let widgets = app.widget_snapshot();
        let quote = widgets
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert_eq!(quote.trim(), "相同 **段落**");
        app.locator(Selector::id("comment_input"))
            .fill("只修改第二个相同段落");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n只修改第二个相同段落");
        app.locator(Selector::id("comment_input"))
            .fill("保留粗体格式");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("message_body").nth(1))
            .wait_text("保留粗体格式");
        app.locator(Selector::id("comment_new")).click();
        app.locator(Selector::id("thread_state"))
            .wait_text("新评论 · 先选择文档段落");
        app.locator(Selector::id("badge").nth(0))
            .wait_visible()
            .click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n只修改第二个相同段落\n\n你\n保留粗体格式");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let screenshot = app.screenshot();
        let pixels = image::open(&screenshot).unwrap().to_rgb8();
        let yellow = pixels
            .pixels()
            .filter(|p| p[0] > 240 && p[1] > 225 && p[2] < 220)
            .count();
        assert!(yellow > 1000, "comment highlight must render yellow pixels");
        println!("highlight+cards: {}", screenshot.display());
        app.locator(Selector::id("thread_resolve_toggle")).click();
        app.locator(Selector::id("thread_state"))
            .wait_text("已解决 · 不再自动处理");
        app.locator(Selector::id("badge").nth(0)).wait_hidden();
        // Reopen by clicking the same checkmark.
        app.locator(Selector::id("thread_resolve_toggle")).click();
        app.locator(Selector::id("badge").nth(0)).wait_visible();
    });
}
#[test]
fn edited_anchor_can_be_explicitly_rebound() {
    support::library::run("rebind_anchor", |app| {
        support::library::fill_document(&app, "原文");
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("comment_input")).fill("完善");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n完善");
        support::library::fill_document(&app, "人工改写内容");
        // Close the select-all popup and leave the source row before testing
        // stale-thread navigation; do not click through a selection overlay.
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("mode_read")).click();
        app.locator(Selector::id("thread_state"))
            .wait_text("原段落已变化 · 请重新绑定");
        // The overview keeps stable thread IDs after a document edit.
        app.locator(Selector::id("thread_open")).click();
        app.locator(Selector::id("rendered").nth(0))
            .wait_visible()
            .click();
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("thread_rebind")).click();
        app.locator(Selector::id("quote")).wait_text("人工改写内容");
        app.locator(Selector::id("transcript"))
            .wait_text("你\n完善");
        // Auto-focus: switching to preview reveals the rebind target.
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("live_editor")).wait_hidden();
        app.locator(Selector::id("badge").nth(0)).wait_visible();
    });
}
