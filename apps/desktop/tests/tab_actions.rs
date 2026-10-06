mod support;
use makepad_test::{MouseButton, RemoteMouseDown, RemoteMouseUp, Selector, StudioToApp, TestApp};
fn menu(app: &TestApp, name: &str) {
    let snap = app.widget_snapshot();
    let tab = snap
        .iter()
        .find(|w| w.id == "title" && w.text.as_deref() == Some(name) && w.visible)
        .unwrap();
    let (x, y) = (tab.x as f64 + 12.0, tab.y as f64 + 12.0);
    app.forward(vec![
        StudioToApp::MouseDown(RemoteMouseDown {
            button_raw_bits: MouseButton::SECONDARY.bits(),
            x,
            y,
            time: 1.0,
            modifiers: Default::default(),
        }),
        StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::SECONDARY.bits(),
            x,
            y,
            time: 1.1,
            modifiers: Default::default(),
        }),
    ]);
    app.locator(Selector::id("tab_close_all")).wait_visible();
    // Trackpad momentum after opening must not dismiss or activate a tab.
    app.forward(vec![StudioToApp::Scroll(
        makepad_widgets::makepad_platform::studio::RemoteScroll {
            x,
            y,
            sx: 30.0,
            sy: 2.0,
            time: 1.2,
            modifiers: Default::default(),
            is_mouse: false,
        },
    )]);
    std::thread::sleep(std::time::Duration::from_millis(150));
    app.locator(Selector::id("tab_close_all")).wait_visible();
}
fn create(app: &TestApp, name: &str) {
    app.locator(Selector::id("new_document")).click();
    app.locator(Selector::id("live_editor"))
        .wait_text("# 未命名文档\n\n");
    support::library::fill_document(app, format!("# {name}\n\n{name} 内容"));
    app.locator(Selector::id("save_label"))
        .wait_text("已自动保存到本地");
}
#[test]
fn menu_survives_opener_release_momentum_and_modifiers_until_explicit_dismissal() {
    support::library::run("tab_menu_lifetime", |app| {
        create(&app, "菜单测试");
        menu(&app, "菜单测试");
        app.press_key(makepad_test::KeyCode::Control);
        app.locator(Selector::id("tab_close_all")).wait_visible();
        app.press_key(makepad_test::KeyCode::Escape);
        app.locator(Selector::id("tab_close_all")).wait_hidden();
        menu(&app, "菜单测试");
        app.locator(Selector::id("title_input")).click();
        app.locator(Selector::id("tab_close_all")).wait_hidden();
        app.locator(Selector::id("title").text_exact("菜单测试"))
            .wait_visible();
    });
}
#[test]
fn conflicting_save_keeps_tabs_and_in_memory_body_on_close_all() {
    support::library::run("tab_close_conflict", |app| {
        create(&app, "冲突测试");
        let root = support::library::root("tab_close_conflict");
        let path = std::fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| {
                p.extension().is_some_and(|e| e == "md")
                    && std::fs::read_to_string(p)
                        .unwrap()
                        .starts_with("# 冲突测试")
            })
            .unwrap();
        std::fs::write(&path, "# 外部编辑\n\n不能覆盖").unwrap();
        support::library::fill_document(&app, "# 冲突测试\n\n内存里的未保存编辑");
        menu(&app, "冲突测试");
        app.locator(Selector::id("tab_close_all")).click();
        app.locator(Selector::id("save_label"))
            .wait_text("自动保存失败 · 保留内存内容");
        app.locator(Selector::id("empty_workspace")).wait_hidden();
        app.locator(Selector::id("title").text_exact("冲突测试"))
            .wait_visible();
        app.locator(Selector::id("live_editor"))
            .assert_text("# 冲突测试\n\n内存里的未保存编辑");
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "# 外部编辑\n\n不能覆盖"
        );
    });
}
#[test]
fn center_tabs_context_scopes_flush_and_empty_workspace_reopens() {
    support::library::run("tab_actions", |app| {
        create(&app, "A");
        create(&app, "B");
        create(&app, "C");
        let snap = app.widget_snapshot();
        let center = snap.iter().find(|w| w.id == "center_panel").unwrap();
        for tab in snap.iter().filter(|w| w.id == "title" && w.visible) {
            assert!(tab.x >= center.x && tab.x + tab.width <= center.x + center.width);
        }
        menu(&app, "B");
        println!("tab context: {}", app.screenshot().display());
        app.locator(Selector::id("tab_close_left")).click();
        app.locator(Selector::id("title").text_exact("A"))
            .wait_hidden();
        app.locator(Selector::id("title").text_exact("B"))
            .wait_visible();
        menu(&app, "B");
        app.locator(Selector::id("tab_close_others")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# B\n\nB 内容");
        app.locator(Selector::id("title").text_exact("C"))
            .wait_hidden();
        menu(&app, "B");
        app.locator(Selector::id("tab_close")).click();
        app.locator(Selector::id("empty_workspace")).wait_visible();
        support::open_settings(&app);
        app.locator(Selector::id("preferences_done")).click();
        app.locator(Selector::id("preferences_overlay"))
            .wait_hidden();
        app.locator(Selector::id("empty_open")).click();
        app.locator(Selector::id("switcher_input")).fill("A");
        let hit = app
            .widget_snapshot()
            .into_iter()
            .find(|w| {
                w.id.starts_with("sw")
                    && w.widget_type == "Button"
                    && w.visible
                    && w.text.as_deref() == Some("A")
            })
            .unwrap();
        app.locator(Selector::id(&hit.id)).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# A\n\nA 内容");
        support::library::fill_document(&app, "# A\n\n尚未自动保存的修改");
        app.locator(Selector::id("save_label"))
            .wait_text("等待自动保存…");
        menu(&app, "A");
        app.locator(Selector::id("tab_close_all")).click();
        app.locator(Selector::id("empty_workspace")).wait_visible();
        let root = support::library::root("tab_actions");
        let contents: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| std::fs::read_to_string(e.path()).unwrap())
            .collect();
        assert!(
            contents.contains(&"# A\n\n尚未自动保存的修改".into()),
            "closing all must flush dirty snapshot"
        );
        app.locator(Selector::id("empty_new")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
    });
}
