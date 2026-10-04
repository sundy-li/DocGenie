use makepad_test::{
    MouseButton, RemoteKeyModifiers, RemoteMouseDown, RemoteMouseUp, Selector, StudioToApp,
    TestApp, TestConfig, run_with_config,
};
use std::path::PathBuf;

pub fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("agent-docs-ui-{}-{name}", std::process::id()))
}
pub fn config(name: &str, root: &std::path::Path) -> TestConfig {
    let mut config =
        TestConfig::current_package(env!("CARGO_MANIFEST_DIR"), env!("CARGO_PKG_NAME"), name)
            .unwrap();
    config
        .env
        .insert("AGENT_DOCS_HOME".into(), root.to_str().unwrap().into());
    config
}
pub fn run(name: &str, test: impl FnOnce(TestApp)) {
    let root = root(name);
    let _ = std::fs::remove_dir_all(&root);
    run_with_config(config(name, &root), |app| {
        super::wait_for_ink(&app, "revision_label");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        test(app);
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(root);
}
pub fn right_click_editor(app: &TestApp) {
    right_click(app, "active_line");
}
pub fn right_click(app: &TestApp, target: &str) {
    let widgets = app.widget_snapshot();
    let editor = widgets.iter().find(|w| w.id == target).unwrap();
    let x = editor.x as f64 + editor.width as f64 * 0.5;
    let y = editor.y as f64 + (editor.height as f64 * 0.5).min(30.0);
    app.forward(vec![
        StudioToApp::MouseDown(RemoteMouseDown {
            button_raw_bits: MouseButton::SECONDARY.bits(),
            x,
            y,
            time: 0.0,
            modifiers: RemoteKeyModifiers::default(),
        }),
        StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::SECONDARY.bits(),
            x,
            y,
            time: 0.1,
            modifiers: RemoteKeyModifiers::default(),
        }),
    ]);
    app.locator(Selector::id("add_comment"))
        .wait_visible()
        .click();
    // New comments open an inline composer first. Legacy sidebar-composer
    // tests explicitly dismiss it and open the existing reply/new-thread UI.
    app.locator(Selector::id("composer_cancel"))
        .wait_visible()
        .click();
    if app
        .widget_snapshot()
        .iter()
        .any(|w| w.id == "expand_comments" && w.visible)
    {
        app.locator(Selector::id("expand_comments")).click();
    }
    app.locator(Selector::id("tab_comments")).click();
}

/// Document editing now uses two-stage select-all; filling a complete file must
/// press twice, unlike comment fields and single-line inputs.
pub fn fill_document(app: &TestApp, text: impl AsRef<str>) -> makepad_test::Locator {
    app.locator(Selector::id("rendered").nth(0))
        .wait_visible()
        .click();
    let modifiers = makepad_test::KeyModifiers {
        logo: true,
        ..Default::default()
    };
    app.press_key_with_modifiers(makepad_test::KeyCode::KeyA, modifiers);
    app.press_key_with_modifiers(makepad_test::KeyCode::KeyA, modifiers);
    // Replace in one native text-input event. Sending Backspace then text
    // races the source row's empty-document relayout and can lose focus.
    app.type_text(text.as_ref());
    app.locator(Selector::id("live_editor"))
        .wait_text(text.as_ref());
    app.locator(Selector::id("live_editor"))
}
