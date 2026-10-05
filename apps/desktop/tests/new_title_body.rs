mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestApp};

fn rename(app: &TestApp, name: &str) {
    app.locator(Selector::id("title_input")).click();
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..Default::default()
        },
    );
    app.type_text(name);
    app.locator(Selector::id("title_input")).wait_value(name);
}
fn click_body(app: &TestApp) {
    let widgets = app.widget_snapshot();
    let body = widgets.iter().find(|w| w.id == "live_editor").unwrap();
    let (x, y) = (body.x as f64 + 28.0, body.y as f64 + 80.0);
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
    app.locator(Selector::id("active_line")).wait_visible();
}
#[test]
fn fresh_title_edits_leave_empty_body_focusable_and_typable() {
    support::library::run("new_title_body", |app| {
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        for title in ["新标题", "再次改标题", "新标题"] {
            rename(&app, title);
            assert!(
                !app.widget_snapshot()
                    .iter()
                    .any(|w| w.id == "title_prefix" && w.visible)
            );
            click_body(&app);
        }
        app.type_text("正文可以输入");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 新标题\n\n正文可以输入");
        app.locator(Selector::id("active_line"))
            .wait_text("正文可以输入");
        rename(&app, "第二标题");
        app.locator(Selector::id("rendered")).click();
        app.press_key(KeyCode::End);
        app.type_text(" space");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 第二标题\n\n正文可以输入 space");
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.type_text("选中后替换");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 第二标题\n\n选中后替换");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.press_key(KeyCode::End);
        app.type_text("继续");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 第二标题\n\n选中后替换继续");
        println!(
            "plain title with editable body: {}",
            app.screenshot().display()
        );
    });
}
