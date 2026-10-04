mod support;
use makepad_test::Selector;
#[test]
fn fenced_script_is_literal_text_not_executable() {
    support::library::run("literal_preview", |app| {
        let source = "# 不可信代码\n\n```runsplash\nLabel{text: \"DO_NOT_EXECUTE\"}\n```";
        support::library::fill_document(&app, source);
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("splash_view"))
            .wait_visible()
            .wait_value("Label{text: \"DO_NOT_EXECUTE\"}\n");
        assert!(
            !app.widget_snapshot()
                .iter()
                .any(|w| w.widget_type == "Label" && w.text.as_deref() == Some("DO_NOT_EXECUTE"))
        );
        support::wait_for_ink(&app, "preview");
    });
}
