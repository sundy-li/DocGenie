mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
};

fn first_row_y(app: &makepad_test::TestApp) -> i64 {
    app.widget_snapshot()
        .iter()
        .filter(|w| w.id == "rendered" || w.id == "active_line")
        .map(|w| w.y)
        .min()
        .unwrap_or(-1)
}

/// Clicking into a paragraph and drag-selecting its text must not scroll the
/// document: selection is not a scrolling gesture.
#[test]
fn selection_does_not_scroll_list() {
    support::library::run("sel_scroll", |app| {
        let doc: String = (1..=30)
            .map(|i| format!("第{i}段内容，一些文字。\n\n"))
            .collect();
        support::library::fill_document(&app, &doc);
        app.locator(Selector::id("comment_input")).click();
        std::thread::sleep(std::time::Duration::from_millis(400));
        let widgets = app.widget_snapshot();
        let mut rows: Vec<_> = widgets
            .iter()
            .filter(|w| w.id == "rendered" && w.visible && w.height > 20)
            .collect();
        rows.sort_by_key(|w| w.y);
        let row = rows[4];
        let x = row.x as f64 + 5.0;
        let y = row.y as f64 + 12.0;
        app.forward(vec![
            StudioToApp::MouseDown(RemoteMouseDown {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 0.5,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseUp(RemoteMouseUp {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 0.6,
                modifiers: Default::default(),
            }),
        ]);
        app.locator(Selector::id("active_line")).wait_visible();
        let y0 = first_row_y(&app);
        app.forward(vec![
            StudioToApp::MouseDown(RemoteMouseDown {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 1.0,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseMove(RemoteMouseMove {
                x: x + 100.0,
                y,
                time: 1.1,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseUp(RemoteMouseUp {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x: x + 100.0,
                y,
                time: 1.2,
                modifiers: Default::default(),
            }),
        ]);
        std::thread::sleep(std::time::Duration::from_millis(600));
        let y1 = first_row_y(&app);
        assert_eq!(y0, y1, "selection must not scroll the document");
    });
}
