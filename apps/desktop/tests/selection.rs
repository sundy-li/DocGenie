mod support;
use makepad_test::{
    KeyCode, KeyModifiers, MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector,
    StudioToApp,
};
fn select(app: &makepad_test::TestApp, logo: bool) {
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo,
            control: !logo,
            ..Default::default()
        },
    );
}
#[test]
fn first_selects_line_second_selects_document_and_replace_autosaves() {
    support::library::run("two_stage_live", |app| {
        let text = "# 标题\n\n第一行\n第二行\n";
        support::library::fill_document(&app, text);
        app.locator(Selector::id("rendered").nth(0)).click();
        app.locator(Selector::id("active_line"))
            .wait_value("第一行");
        select(&app, false);
        app.locator(Selector::id("add_comment")).wait_visible();
        println!("selection toolbar: {}", app.screenshot().display());
        select(&app, false);
        app.locator(Selector::id("active_line")).wait_value(text);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("quote")).wait_value(text);
        // Full native selection, not just a highlighted UI approximation.
        app.locator(Selector::id("rendered").nth(0)).click();
        select(&app, true);
        select(&app, true);
        app.locator(Selector::id("active_line")).wait_value(text);
        app.type_text("# 替换全文\n\n新正文");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 替换全文\n\n新正文");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
    });
}
#[test]
fn source_two_stage_select_and_first_press_replaces_only_current_line() {
    support::library::run("two_stage_source", |app| {
        support::library::fill_document(&app, "前行\n当前行\n后行");
        app.locator(Selector::id("rendered").nth(0)).click();
        app.press_key_with_modifiers(
            KeyCode::Home,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        select(&app, false);
        select(&app, false);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("quote"))
            .wait_value("前行\n当前行\n后行");
        app.locator(Selector::id("rendered").nth(0)).click();
        select(&app, true);
        app.type_text("替换行");
        let widgets = app.widget_snapshot();
        let value = widgets
            .iter()
            .find(|w| w.id == "live_editor")
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert_eq!(value.lines().count(), 3);
        assert!(value.contains("替换行"));
    });
}
#[test]
fn mouse_drag_shows_comment_toolbar_without_right_click() {
    support::library::run("selection_toolbar_drag", |app| {
        support::library::fill_document(&app, "可以用鼠标选中这段文字来添加评论");
        let widgets = app.widget_snapshot();
        let editor = widgets.iter().find(|w| w.id == "active_line").unwrap();
        let x = editor.x as f64 + 8.0;
        let y = editor.y as f64 + 16.0;
        app.forward(vec![
            StudioToApp::MouseDown(RemoteMouseDown {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 0.0,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseMove(RemoteMouseMove {
                x: x + 130.0,
                y,
                time: 0.1,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseUp(RemoteMouseUp {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x: x + 130.0,
                y,
                time: 0.2,
                modifiers: Default::default(),
            }),
        ]);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("comment_input"))
            .fill("直接从浮动工具条评论");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n直接从浮动工具条评论");
        println!("toolbar comment: {}", app.screenshot().display());
    });
}

#[test]
fn selection_toolbar_applies_and_toggles_markdown_styles() {
    support::library::run("format_toolbar", |app| {
        support::library::fill_document(&app, "选中这段文字加粗");
        // The fill leaves the row active and focused: select the whole line.
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("fmt_bold")).wait_visible().click();
        app.locator(Selector::id("live_editor"))
            .wait_text("**选中这段文字加粗**");
        // Re-select and toggle bold off again.
        app.locator(Selector::id("rendered")).click();
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("fmt_bold")).wait_visible().click();
        app.locator(Selector::id("live_editor"))
            .wait_text("选中这段文字加粗");
    });
}
