mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
};

fn ev_move(x: f64, y: f64, t: f64) -> StudioToApp {
    StudioToApp::MouseMove(RemoteMouseMove {
        x,
        y,
        time: t,
        modifiers: Default::default(),
    })
}

/// Drag across wrapped visual lines of an ALREADY-ACTIVE row (native TextInput
/// path): the caret must keep tracking the pointer into the second visual line.
#[test]
fn active_row_wrap_drag_tracks_pointer() {
    support::library::run("active_wrap_drag", |app| {
        let paragraph = "导读：Jev 最近很火，它擅长的语义判断如果直接放进数据库，能否让分类、筛选这类任务留在 SQL 里完成？本文以 Databend 的 Python UDF 集成为例，走通接入和查询过程，也看看远程模型调用的性能、成本与数据边界。";
        support::library::fill_document(&app, paragraph);
        app.locator(Selector::id("comment_input")).click();
        // Activate the row (rendered -> source TextInput).
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line")).wait_visible();
        let widgets = app.widget_snapshot();
        let row = widgets.iter().find(|w| w.id == "active_line").unwrap();
        println!(
            "active_line rect: x={} y={} w={} h={}",
            row.x, row.y, row.width, row.height
        );
        assert!(
            row.height > 50,
            "paragraph should wrap to multiple visual lines"
        );
        let x0 = row.x as f64 + 5.0;
        let y0 = row.y as f64 + 12.0;
        let y2 = row.y as f64 + row.height as f64 - 8.0; // last visual line
        app.forward(vec![StudioToApp::MouseDown(RemoteMouseDown {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x: x0,
            y: y0,
            time: 1.0,
            modifiers: Default::default(),
        })]);
        // Several small steps along line 1, then diagonal into the last line.
        let mut t = 1.1;
        for (x, y) in [
            (x0 + 60.0, y0),
            (x0 + 120.0, y0),
            (x0 + 180.0, y0 + 20.0),
            (x0 + 240.0, y2),
        ] {
            app.forward(vec![ev_move(x, y, t)]);
            t += 0.1;
        }
        app.forward(vec![StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x: x0 + 240.0,
            y: y2,
            time: t,
            modifiers: Default::default(),
        })]);
        println!("after drag: {}", app.screenshot().display());
        app.locator(Selector::id("add_comment")).wait_visible();
        // The toolbar must persist, not flash and vanish: let blink timers,
        // the save debounce and pending signals fire, then re-check.
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let still_visible = app
            .widget_snapshot()
            .iter()
            .find(|w| w.id == "add_comment")
            .map(|w| w.visible)
            .unwrap_or(false);
        assert!(
            still_visible,
            "comment toolbar disappeared after the selection settled"
        );
        app.locator(Selector::id("add_comment")).click();
        app.locator(Selector::id("add_comment")).wait_hidden();
        let snapshot = app.widget_snapshot();
        let quote = snapshot
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .text
            .as_deref()
            .unwrap()
            .to_string();
        println!("quote: {quote}");
        assert!(!quote.is_empty());
        // The drag ended on the last visual line, so the selection must reach
        // past the first visual line of the wrapped paragraph.
        assert!(
            quote.contains("类任务留在") || quote.len() > 60,
            "selection did not follow the pointer into wrapped lines: {quote:?}"
        );
    });
}
