mod support;
use makepad_test::{
    MouseButton, RemoteMouseDown, RemoteMouseMove, RemoteMouseUp, Selector, StudioToApp,
    TestConfig, run_with_config,
};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

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
fn inline_composer_creates_plain_comment() {
    support::library::run("composer_plain", |app| {
        support::library::fill_document(&app, "选这段文字来评论");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).wait_visible();
        drag(&app, "rendered");
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        // Inline card opens with the quote and the per-comment AI switch.
        {
            let snapshot = app.widget_snapshot();
            let quote = snapshot
                .iter()
                .find(|w| w.id == "composer_quote")
                .and_then(|w| w.value.as_deref())
                .unwrap_or("")
                .to_string();
            assert!(quote.starts_with("选这段文字"), "composer quote: {quote:?}");
        }
        app.locator(Selector::id("composer_ask_ai")).wait_visible();
        app.locator(Selector::id("composer_input")).fill("普通评论");
        app.locator(Selector::id("composer_submit")).click();
        app.locator(Selector::id("composer_input")).wait_hidden();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n普通评论");
        // 未勾选问 AI:文档不变,Agent 不触发。
        app.locator(Selector::id("live_editor"))
            .assert_text("选这段文字来评论");
        println!("composer plain: {}", app.screenshot().display());
    });
}

#[test]
fn inline_composer_ask_ai_triggers_agent_without_global_default() {
    // One-shot fake endpoint: observes exactly one agent request.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        let header_end = loop {
            let count = stream.read(&mut chunk).unwrap_or(0);
            if count == 0 {
                break 0;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
        };
        assert!(header_end > 0, "agent request headers");
        let length = String::from_utf8_lossy(&bytes[..header_end])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        // Consume the complete request before closing the socket; unread body
        // bytes can reset the connection under the slower debug App.
        while bytes.len() < header_end + length {
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0, "complete mock request body");
            bytes.extend_from_slice(&chunk[..count]);
        }
        let edit =
            serde_json::json!({"replacement": "AI 改写后的段落", "explanation": "已按评论改写"});
        let body =
            serde_json::json!({"choices":[{"message":{"content": edit.to_string()}}]}).to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let root = support::library::root("composer_ai");
    let _ = std::fs::remove_dir_all(&root);
    let mut config = TestConfig::current_package(
        env!("CARGO_MANIFEST_DIR"),
        env!("CARGO_PKG_NAME"),
        "composer_ai",
    )
    .unwrap();
    config
        .env
        .insert("AGENT_DOCS_HOME".into(), root.to_str().unwrap().into());
    config.env.insert("AGENT_DOCS_ENDPOINT".into(), endpoint);
    config
        .env
        .insert("AGENT_DOCS_MODEL".into(), "test-only".into());
    config
        .env
        .insert("AGENT_DOCS_API_KEY".into(), String::new());
    run_with_config(config, |app| {
        support::wait_for_ink(&app, "revision_label");
        support::library::fill_document(&app, "原始段落");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).wait_visible();
        drag(&app, "rendered");
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("composer_ask_ai")).click();
        app.locator(Selector::id("composer_input")).fill("帮我润色");
        app.locator(Selector::id("composer_submit")).click();
        // 全局默认关闭,但这条评论勾选了「问 AI」:Agent 仍处理并改写。
        app.locator(Selector::id("live_editor"))
            .wait_text("AI 改写后的段落");
        app.locator(Selector::id("transcript"))
            .wait_text("你\n帮我润色\n\nAgent\n已按评论改写");
    })
    .unwrap();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn commented_row_paints_yellow_in_edit_mode() {
    support::library::run("edit_highlight", |app| {
        support::library::fill_document(&app, "第一段文字\n\n第二段文字");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).wait_visible();
        drag(&app, "rendered");
        app.locator(Selector::id("add_comment"))
            .wait_visible()
            .click();
        app.locator(Selector::id("composer_input")).fill("黄底评论");
        app.locator(Selector::id("composer_submit")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n黄底评论");
        // 编辑模式下,有未解决评论的行渲染黄底。
        let screenshot = app.screenshot();
        let pixels = image::open(&screenshot).unwrap().to_rgb8();
        let widgets = app.widget_snapshot();
        let panel = widgets.iter().find(|w| w.id == "live_panel").unwrap();
        let root = widgets.iter().find(|w| w.id == "main_window").unwrap();
        let scale = pixels.width() as f64 / root.width as f64;
        let (px, py) = (panel.x as f64 * scale, panel.y as f64 * scale);
        let (pw, ph) = (panel.width as f64 * scale, panel.height as f64 * scale);
        let mut yellow = 0;
        for y in (py as u32)..((py + ph) as u32).min(pixels.height()) {
            for x in (px as u32)..((px + pw) as u32).min(pixels.width()) {
                let p = pixels.get_pixel(x, y).0;
                if p[0] > 240 && p[1] > 225 && p[2] < 220 {
                    yellow += 1;
                }
            }
        }
        assert!(
            yellow > 500,
            "commented row must paint yellow, got {yellow}"
        );
        println!("edit highlight: {}", screenshot.display());
    });
}
