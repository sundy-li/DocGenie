mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
};
use makepad_widgets::makepad_platform::studio::RemoteScroll;
#[test]
fn selection_center_blocks_scroll_edges_allow_and_release_stops() {
    support::library::run("selection_edges", |app| {
        let body = (0..60)
            .map(|i| format!("段落 {i}，用于验证拖选边缘滚动。\n\n"))
            .collect::<String>();
        support::library::fill_document(&app, &body);
        app.locator(Selector::id("comment_input")).click();
        let snapshot = app.widget_snapshot();
        let viewport = snapshot
            .iter()
            .find(|w| w.id == "live_editor")
            .unwrap()
            .clone();
        let mut rows: Vec<_> = snapshot
            .iter()
            .filter(|w| w.id == "rendered" && w.visible && w.height > 20)
            .collect();
        rows.sort_by_key(|w| w.y);
        let row = rows[rows.len() / 2];
        let (x, y) = (row.x as f64 + 10.0, row.y as f64 + 10.0);
        app.forward(vec![StudioToApp::MouseDown(RemoteMouseDown {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x,
            y,
            time: 1.0,
            modifiers: Default::default(),
        })]);
        app.locator(Selector::id("active_line")).wait_visible();
        app.forward(vec![StudioToApp::MouseMove(RemoteMouseMove {
            x: x + 70.0,
            y,
            time: 1.1,
            modifiers: Default::default(),
        })]);
        let before = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "active_line" && w.visible)
            .unwrap()
            .y;
        app.forward(vec![StudioToApp::Scroll(RemoteScroll {
            x,
            y,
            sx: 0.0,
            sy: 100.0,
            time: 1.2,
            modifiers: Default::default(),
            is_mouse: true,
        })]);
        std::thread::sleep(std::time::Duration::from_millis(350));
        let middle = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "active_line" && w.visible)
            .unwrap()
            .y;
        assert_eq!(middle, before, "middle selection must not enter scrolling");
        let top = viewport.y as f64 + 2.0;
        app.forward(vec![StudioToApp::MouseMove(RemoteMouseMove {
            x,
            y: top,
            time: 1.4,
            modifiers: Default::default(),
        })]);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let edged = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "active_line" && w.visible)
            .unwrap()
            .y;
        assert!(
            edged > before,
            "top edge must allow scrolling toward earlier paragraphs"
        );
        let bottom = viewport.y as f64 + viewport.height as f64 - 2.0;
        app.forward(vec![StudioToApp::MouseMove(RemoteMouseMove {
            x,
            y: bottom,
            time: 1.6,
            modifiers: Default::default(),
        })]);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let bottom_y = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "active_line" && w.visible)
            .unwrap()
            .y;
        assert!(
            bottom_y < edged,
            "bottom edge must scroll toward later paragraphs"
        );
        app.forward(vec![StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x,
            y: bottom,
            time: 1.7,
            modifiers: Default::default(),
        })]);
        let stopped = app
            .widget_snapshot()
            .into_iter()
            .find(|w| w.id == "active_line" && w.visible)
            .unwrap()
            .y;
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert_eq!(
            app.widget_snapshot()
                .into_iter()
                .find(|w| w.id == "active_line" && w.visible)
                .unwrap()
                .y,
            stopped,
            "release must stop selection scroll"
        );
    });
}
