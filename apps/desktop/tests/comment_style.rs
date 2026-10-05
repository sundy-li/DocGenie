mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, run_with_config};
#[test]
fn quote_is_grey_abbreviated_clickable_and_message_timestamp_survives_restart() {
    let root = support::library::root("comment_style_restart");
    let _ = std::fs::remove_dir_all(&root);
    let source = format!(
        "# 评论样式\n\n{}",
        "这是需要引用的一段原文，摘要应当缩略显示。".repeat(6)
    );
    let time_text = std::cell::RefCell::new(String::new());
    run_with_config(
        support::library::config("comment_style_create", &root),
        |app| {
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
            support::wait_for_ink(&app, "revision_label");
            support::library::fill_document(&app, &source);
            app.locator(Selector::id("comment_input")).click();
            app.locator(Selector::id("rendered")).click();
            app.press_key_with_modifiers(
                KeyCode::KeyA,
                KeyModifiers {
                    logo: true,
                    ..Default::default()
                },
            );
            app.press_key_with_modifiers(
                KeyCode::KeyM,
                KeyModifiers {
                    logo: true,
                    shift: true,
                    ..Default::default()
                },
            );
            app.locator(Selector::id("composer_input"))
                .fill("这是一条评论");
            app.locator(Selector::id("composer_submit")).click();
            let snapshot = app.widget_snapshot();
            let excerpt = snapshot
                .iter()
                .find(|w| w.id == "quote_jump")
                .unwrap()
                .text
                .clone()
                .unwrap();
            assert!(excerpt.ends_with('…') && excerpt.chars().count() <= 47);
            app.locator(Selector::id("quote"))
                .assert_text(source.split_once("\n\n").unwrap().1);
            app.locator(Selector::id("author").text_exact("你"))
                .wait_visible();
            *time_text.borrow_mut() = snapshot
                .iter()
                .find(|w| w.id == "message_time" && w.visible)
                .unwrap()
                .text
                .clone()
                .unwrap();
            assert!(time_text.borrow().contains('月') && time_text.borrow().contains(':'));
            app.locator(Selector::id("quote_jump")).click();
            app.locator(Selector::id("active_line")).wait_visible();
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
            println!("comment card style: {}", app.screenshot().display());
        },
    )
    .unwrap();
    run_with_config(
        support::library::config("comment_style_restore", &root),
        |app| {
            app.locator(Selector::id("save_label"))
                .wait_text("已自动保存到本地");
            app.locator(Selector::id("message_time"))
                .wait_text(time_text.borrow().as_str());
            app.locator(Selector::id("author").text_exact("你"))
                .wait_visible();
            app.locator(Selector::id("live_editor"))
                .assert_text(&source);
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
