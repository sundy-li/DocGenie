mod support;
use makepad_test::Selector;

/// Clicking an empty row between two paragraphs and typing must insert text
/// (the placeholder row used to swallow keys intermittently).
#[test]
fn typing_into_empty_middle_row_works() {
    support::library::run("empty_row_type", |app| {
        support::library::fill_document(&app, "第一段\n\n\n第三段");
        app.locator(Selector::id("comment_input")).click();
        let widgets = app.widget_snapshot();
        let mut rows: Vec<_> = widgets.iter().filter(|w| w.id == "rendered").collect();
        rows.sort_by_key(|w| w.y);
        assert_eq!(
            rows.len(),
            2,
            "two paragraphs; the empty line renders as a blank strip"
        );
        // Click the blank strip between the two rendered rows.
        let x = rows[0].x as f64 + 24.0;
        let y = (rows[0].y + rows[0].height) as f64 + 12.0;
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
        // The clicked blank line becomes the typed line; one paragraph gap
        // newline is consumed, which is the standard click-and-type result.
        app.type_text("第二段");
        app.locator(Selector::id("live_editor"))
            .wait_text("第一段\n第二段\n\n第三段");
        println!("empty row typing: {}", app.screenshot().display());
    });
}

/// Keys sent immediately after the press (before the activation draw settles)
/// must land in the freshly activated row as well.
#[test]
fn typing_immediately_after_press_works() {
    support::library::run("instant_type", |app| {
        support::library::fill_document(&app, "第一段\n\n第三段");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered").nth(1)).click();
        // No wait for the active row: type right away.
        // Activation places the caret at the row start, so the typed text
        // prepends into the clicked row.
        app.type_text("马上输入");
        app.locator(Selector::id("live_editor"))
            .wait_text("第一段\n\n马上输入第三段");
    });
}
