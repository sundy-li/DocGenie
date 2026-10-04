mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
};

fn drag(app: &makepad_test::TestApp, target: &str) {
    let widgets = app.widget_snapshot();
    let widget = widgets.iter().find(|w| w.id == target).unwrap();
    let y = widget.y as f64 + 12.0;
    let left = widget.x as f64 + 5.0;
    let right = left + 112.0;
    app.forward(vec![
        StudioToApp::MouseDown(RemoteMouseDown {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x: left,
            y,
            time: 1.0,
            modifiers: Default::default(),
        }),
        StudioToApp::MouseMove(RemoteMouseMove {
            x: right,
            y,
            time: 1.2,
            modifiers: Default::default(),
        }),
        StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x: right,
            y,
            time: 1.3,
            modifiers: Default::default(),
        }),
    ]);
}

#[test]
fn toolbar_stays_visible_after_selection() {
    support::library::run("toolbar_stays", |app| {
        support::library::fill_document(&app, "可以用鼠标选中这段文字来添加评论");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).wait_visible();
        drag(&app, "rendered");
        app.locator(Selector::id("add_comment")).wait_visible();
        println!("fresh: {}", app.screenshot().display());
        // Let real time pass: blink timers, save debounce, signals all fire.
        std::thread::sleep(std::time::Duration::from_millis(1600));
        let widgets = app.widget_snapshot();
        let visible = widgets
            .iter()
            .find(|w| w.id == "add_comment")
            .map(|w| w.visible)
            .unwrap_or(false);
        println!("after 1.6s visible={visible}");
        println!("after: {}", app.screenshot().display());
        assert!(visible, "comment toolbar disappeared 1.6s after selection");
    });
}
