mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, TestConfig, run_with_config};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};

#[test]
fn configured_transport_edits_and_replies_for_two_rounds() {
    // Local fake HTTP service: verifies real transport and automatic apply;
    // intentionally NOT evidence of a live model's writing quality.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let server = std::thread::spawn(move || {
        let mut round = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while round < 2 {
            let mut stream = loop {
                if let Ok((s, _)) = listener.accept() {
                    break s;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "missing authorized request"
                );
                std::thread::sleep(Duration::from_millis(20));
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
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
            if header_end == 0 {
                continue;
            }
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let len: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap();
            while bytes.len() < header_end + len {
                let count = stream.read(&mut chunk).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&chunk[..count]);
            }
            let wire: serde_json::Value =
                serde_json::from_slice(&bytes[header_end..header_end + len]).unwrap();
            assert_eq!(wire["model"], "test-only");
            let context: serde_json::Value =
                serde_json::from_str(wire["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(
                context["paragraph"],
                if round == 0 {
                    "原始段落"
                } else {
                    "第一轮修改"
                }
            );
            assert_eq!(
                context["comments"].as_array().unwrap().len(),
                if round == 0 { 1 } else { 3 }
            );
            let edit = serde_json::json!({"replacement": if round == 0 { "第一轮修改" } else { "第二轮完善" }, "explanation": if round == 0 { "已补充例子" } else { "已按回复精简" }});
            let response =
                serde_json::json!({"choices":[{"message":{"content":edit.to_string()}}]})
                    .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
            round += 1;
        }
    });
    let root = support::library::root("agent_two_rounds");
    let _ = std::fs::remove_dir_all(&root);
    let mut config = TestConfig::current_package(
        env!("CARGO_MANIFEST_DIR"),
        env!("CARGO_PKG_NAME"),
        "agent_two_rounds",
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
        app.press_key_with_modifiers(
            KeyCode::KeyA,
            KeyModifiers {
                logo: true,
                ..KeyModifiers::default()
            },
        );
        support::library::right_click_editor(&app);
        app.locator(Selector::id("comment_input"))
            .fill("补充一个例子");
        // Open Preferences and toggle default_auto_modify so the upcoming
        // comment_send triggers the agent automatically.
        app.locator(Selector::id("preferences_button")).click();
        app.locator(Selector::id("pref_default_auto")).click();
        app.locator(Selector::id("preferences_save")).click();
        app.locator(Selector::id("preferences_overlay"))
            .wait_hidden();
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("第一轮修改");
        app.locator(Selector::id("transcript"))
            .wait_text("你\n补充一个例子\n\nAgent\n已补充例子");
        app.locator(Selector::id("comment_input"))
            .fill("再简短一点");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("第二轮完善");
        app.locator(Selector::id("transcript")).wait_text(
            "你\n补充一个例子\n\nAgent\n已补充例子\n\n你\n再简短一点\n\nAgent\n已按回复精简",
        );
        app.locator(Selector::id("undo_button")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("第一轮修改");
    })
    .unwrap();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
