mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestApp};

fn select_all(app: &TestApp) {
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..Default::default()
        },
    );
}

#[test]
fn plain_document_title_never_adds_prefix_or_moves_the_caret() {
    support::library::run("title_prefix_typing", |app| {
        support::library::fill_document(&app, "# 原标题\n\n开始写作。");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("title_input")).click();
        select_all(&app);
        app.press_key(KeyCode::Backspace);
        app.type_text("f");
        app.locator(Selector::id("title_input")).wait_value("f");
        // Saving and the focus transitions must not insert a synthetic '# '
        // into the native buffer or reset its cursor / undo history.
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.type_text("f");
        app.locator(Selector::id("title_input")).wait_value("ff");
        app.locator(Selector::id("live_editor"))
            .wait_text("# ff\n\n开始写作。");
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "title_prefix" && w.visible)
        );
        app.press_key(KeyCode::Home);
        app.type_text("中");
        app.locator(Selector::id("title_input")).wait_value("中ff");
        app.press_key(KeyCode::ArrowRight);
        app.type_text("文");
        app.locator(Selector::id("title_input"))
            .wait_value("中f文f");
        app.press_key_with_modifiers(
            KeyCode::KeyZ,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("title_input")).wait_value("中ff");
        app.locator(Selector::id("comment_input")).click();
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "title_prefix" && w.visible)
        );
        app.locator(Selector::id("title_input")).wait_value("中ff");
        app.locator(Selector::id("title_input")).click();
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.id == "title_prefix" && w.visible)
        );
        app.locator(Selector::id("title_input")).wait_value("中ff");
        app.press_key(KeyCode::End);
        app.type_text("f");
        app.locator(Selector::id("title_input")).wait_value("中fff");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 中fff\n\n开始写作。");
        // A trailing space must not be normalized back into the focused
        // buffer between keystrokes; the following character belongs after it.
        app.type_text(" ");
        app.locator(Selector::id("title_input"))
            .wait_value("中fff ");
        app.type_text("f");
        app.locator(Selector::id("title_input"))
            .wait_value("中fff f");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 中fff f\n\n开始写作。");
        println!("isolated heading prefix: {}", app.screenshot().display());
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        let root = support::library::root("title_prefix_typing");
        let file = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|ext| ext == "md"))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(file).unwrap(),
            "# 中fff f\n\n开始写作。"
        );
    });
}
