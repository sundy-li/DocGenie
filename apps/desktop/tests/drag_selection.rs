mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
};

fn drag(app: &makepad_test::TestApp, target: &str, backwards: bool, batched: bool) {
    let widgets = app.widget_snapshot();
    let widget = widgets.iter().find(|w| w.id == target).unwrap();
    let y = widget.y as f64 + 12.0;
    let left = widget.x as f64 + 5.0;
    let right = left + 112.0;
    let (start, end) = if backwards {
        (right, left)
    } else {
        (left, right)
    };
    let down = StudioToApp::MouseDown(RemoteMouseDown {
        button_raw_bits: MouseButton::PRIMARY.bits(),
        x: start,
        y,
        time: 1.0,
        modifiers: Default::default(),
    });
    let movement = StudioToApp::MouseMove(RemoteMouseMove {
        x: end,
        y,
        time: 1.2,
        modifiers: Default::default(),
    });
    let up = StudioToApp::MouseUp(RemoteMouseUp {
        button_raw_bits: MouseButton::PRIMARY.bits(),
        x: end,
        y,
        time: 1.3,
        modifiers: Default::default(),
    });
    if batched {
        app.forward(vec![down, movement, up]);
    } else {
        app.forward(vec![down]);
        app.locator(Selector::id("active_line")).wait_visible();
        app.forward(vec![movement, up]);
    }
}
#[test]
fn first_drag_on_rendered_line_preserves_selection() {
    support::library::run("first_rendered_drag", |app| {
        support::library::fill_document(&app, "鼠标拖动选择这一段文字，不需要先点击再拖。");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).wait_visible();
        drag(&app, "rendered", false, false);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("add_comment")).wait_hidden();
        let snapshot = app.widget_snapshot();
        let quote = snapshot
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .value
            .as_deref()
            .unwrap();
        assert!(!quote.is_empty());
        assert!("鼠标拖动选择这一段文字，不需要先点击再拖。".starts_with(quote));
        app.locator(Selector::id("comment_input"))
            .fill("鼠标拖选的批注");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n鼠标拖选的批注");
        app.locator(Selector::id("live_editor"))
            .assert_text("鼠标拖动选择这一段文字，不需要先点击再拖。");
    });
}
#[test]
fn fast_reverse_drag_on_rendered_line_preserves_selection() {
    support::library::run("reverse_rendered_drag", |app| {
        support::library::fill_document(&app, "从右向左拖动选择文字也必须成功。");
        app.locator(Selector::id("comment_input")).click();
        drag(&app, "rendered", true, true);
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("add_comment")).wait_hidden();
        let snapshot = app.widget_snapshot();
        let quote = snapshot
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .value
            .as_deref()
            .unwrap();
        assert!(!quote.is_empty());
        assert!("从右向左拖动选择文字也必须成功。".contains(quote));
    });
}

#[test]
fn wrapped_unicode_paragraph_drag_preserves_neighbors() {
    support::library::run("wrapped_drag", |app| {
        let paragraph = "中文👋与组合字符 é 应该按字形选择，长段落换行后也不能丢失选区。".repeat(8);
        let full = format!("前段\n\n{}\n\n后段", paragraph);
        support::library::fill_document(&app, &full);
        let widgets = app.widget_snapshot();
        let mut rows: Vec<_> = widgets.iter().filter(|w| w.id == "rendered").collect();
        rows.sort_by_key(|w| w.y);
        let row = rows[1];
        assert!(row.height > 50);
        let x = row.x as f64 + 5.0;
        let y = row.y as f64 + 12.0;
        app.forward(vec![
            StudioToApp::MouseDown(RemoteMouseDown {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x,
                y,
                time: 2.0,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseMove(RemoteMouseMove {
                x: x + 100.0,
                y: y + 40.0,
                time: 2.1,
                modifiers: Default::default(),
            }),
            StudioToApp::MouseUp(RemoteMouseUp {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x: x + 100.0,
                y: y + 40.0,
                time: 2.2,
                modifiers: Default::default(),
            }),
        ]);
        app.locator(Selector::id("add_comment")).wait_visible();
        println!("wrapped drag selection: {}", app.screenshot().display());
        app.locator(Selector::id("add_comment")).click();
        app.locator(Selector::id("add_comment")).wait_hidden();
        let snapshot = app.widget_snapshot();
        let quote = snapshot
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .value
            .as_deref()
            .unwrap();
        assert!(!quote.is_empty() && quote.len() < paragraph.len());
        assert!(paragraph.contains(quote));
        app.locator(Selector::id("live_editor")).assert_text(&full);
    });
}
#[test]
fn active_source_drag_can_be_reselected_and_replaced() {
    support::library::run("active_drag", |app| {
        support::library::fill_document(&app, "源段落，先点击再拖动也必须正常。");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line")).wait_visible();
        drag(&app, "active_line", false, true);
        app.locator(Selector::id("add_comment")).wait_visible();
        app.type_text("替换选区");
        app.locator(Selector::id("add_comment")).wait_hidden();
        let snapshot = app.widget_snapshot();
        let text = snapshot
            .iter()
            .find(|w| w.id == "live_editor")
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert!(text.contains("替换选区"));
        assert!(text.ends_with("也必须正常。"));
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
    });
}

/// Double-click selects a word; dragging from there must keep extending the
/// selection across wrapped visual lines instead of freezing at the word
/// boundary (the native multi-click drag path drops those moves).
#[test]
fn double_click_drag_extends_across_wrapped_lines() {
    support::library::run("double_click_wrap_drag", |app| {
        let paragraph = "回答者的判断是：把「高精力」等同于早睡早起、健身、吃补剂，是在回避重点。提高精力的关键是让大脑给你批更多「算力预算」。".repeat(2);
        support::library::fill_document(&app, &paragraph);
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line")).wait_visible();
        let widgets = app.widget_snapshot();
        let row = widgets.iter().find(|w| w.id == "active_line").unwrap();
        assert!(row.height > 50, "paragraph should wrap to multiple lines");
        let x0 = row.x as f64 + 20.0;
        let y0 = row.y as f64 + 12.0;
        let y2 = row.y as f64 + row.height as f64 - 8.0;
        let down = |t: f64| {
            StudioToApp::MouseDown(RemoteMouseDown {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x: x0,
                y: y0,
                time: t,
                modifiers: Default::default(),
            })
        };
        // First press: plain caret placement.
        app.forward(vec![
            down(1.0),
            StudioToApp::MouseUp(RemoteMouseUp {
                button_raw_bits: MouseButton::PRIMARY.bits(),
                x: x0,
                y: y0,
                time: 1.1,
                modifiers: Default::default(),
            }),
        ]);
        // Second press inside the double-click window, then drag diagonally
        // into the last visual line.
        app.forward(vec![down(1.2)]);
        app.forward(vec![StudioToApp::MouseMove(RemoteMouseMove {
            x: x0 + 300.0,
            y: y2,
            time: 1.3,
            modifiers: Default::default(),
        })]);
        app.forward(vec![StudioToApp::MouseUp(RemoteMouseUp {
            button_raw_bits: MouseButton::PRIMARY.bits(),
            x: x0 + 300.0,
            y: y2,
            time: 1.4,
            modifiers: Default::default(),
        })]);
        app.locator(Selector::id("add_comment")).wait_visible();
        app.locator(Selector::id("add_comment")).click();
        let snapshot = app.widget_snapshot();
        let quote = snapshot
            .iter()
            .find(|w| w.id == "quote")
            .unwrap()
            .value
            .as_deref()
            .unwrap()
            .to_string();
        assert!(
            quote.contains("算力预算"),
            "double-click drag froze at the word boundary: {quote:?}"
        );
    });
}
