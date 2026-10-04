mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector, run_with_config};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

fn service(
    rounds: usize,
) -> (
    String,
    mpsc::Receiver<serde_json::Value>,
    mpsc::Sender<()>,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let (request_tx, request_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut round = 0;
        while round < rounds {
            let mut stream = loop {
                if let Ok((s, _)) = listener.accept() {
                    break s;
                }
                assert!(Instant::now() < deadline, "missing request");
                std::thread::sleep(Duration::from_millis(20));
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            let end = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break 0;
                }
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            if end == 0 {
                continue;
            }
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let size: usize = headers
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap();
            while bytes.len() < end + size {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
            }
            let body: serde_json::Value = serde_json::from_slice(&bytes[end..end + size]).unwrap();
            request_tx.send(body).unwrap();
            release_rx.recv_timeout(Duration::from_secs(30)).unwrap();
            let edit = serde_json::json!({"replacement": "处理后的段落", "explanation":"已按最新评论修改"});
            let body = serde_json::json!({"choices":[{"message":{"content": edit.to_string()}}]})
                .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            round += 1;
        }
    });
    (endpoint, request_rx, release_tx, server)
}
fn prepare(app: &makepad_test::TestApp) {
    support::wait_for_ink(app, "revision_label");
    app.locator(Selector::id("save_label"))
        .wait_text("已自动保存到本地");
    support::library::fill_document(app, "原文");
    app.press_key_with_modifiers(
        KeyCode::KeyA,
        KeyModifiers {
            logo: true,
            ..KeyModifiers::default()
        },
    );
    support::library::right_click_editor(app);
    app.locator(Selector::id("comment_input")).fill("改善原文");
    // Enable Agent auto-modify via Preferences before sending the comment
    // so the very first send triggers a request the mock server can observe.
    app.locator(Selector::id("preferences_button")).click();
    app.locator(Selector::id("pref_default_auto")).click();
    app.locator(Selector::id("preferences_save")).click();
    app.locator(Selector::id("preferences_overlay"))
        .wait_hidden();
    app.locator(Selector::id("comment_send")).click();
}
#[test]
fn stop_discards_late_response_without_writing() {
    let root = support::library::root("stop_late");
    let _ = std::fs::remove_dir_all(&root);
    let (endpoint, requests, release, server) = service(1);
    let mut config = support::library::config("stop_late", &root);
    config.env.insert("AGENT_DOCS_ENDPOINT".into(), endpoint);
    config
        .env
        .insert("AGENT_DOCS_MODEL".into(), "test-only".into());
    config
        .env
        .insert("AGENT_DOCS_API_KEY".into(), String::new());
    run_with_config(config, |app| {
        prepare(&app);
        requests.recv_timeout(Duration::from_secs(10)).unwrap();
        app.locator(Selector::id("agent_cancel")).click();
        release.send(()).unwrap();
        app.locator(Selector::id("status_label"))
            .wait_text("结果已忽略，文档未修改");
        app.locator(Selector::id("live_editor")).assert_text("原文");
        app.locator(Selector::id("transcript"))
            .assert_text("你\n改善原文");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
    })
    .unwrap();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
#[test]
fn newer_reply_queues_latest_round_and_invalidates_old_result() {
    let root = support::library::root("newer_round");
    let _ = std::fs::remove_dir_all(&root);
    let (endpoint, requests, release, server) = service(2);
    let mut config = support::library::config("newer_round", &root);
    config.env.insert("AGENT_DOCS_ENDPOINT".into(), endpoint);
    config
        .env
        .insert("AGENT_DOCS_MODEL".into(), "test-only".into());
    config
        .env
        .insert("AGENT_DOCS_API_KEY".into(), String::new());
    run_with_config(config, |app| {
        prepare(&app);
        requests.recv_timeout(Duration::from_secs(10)).unwrap();
        app.locator(Selector::id("comment_input"))
            .fill("按新增要求修改");
        app.locator(Selector::id("comment_send")).click();
        app.locator(Selector::id("transcript"))
            .wait_text("你\n改善原文\n\n你\n按新增要求修改");
        release.send(()).unwrap();
        let body = requests.recv_timeout(Duration::from_secs(10)).unwrap();
        let context: serde_json::Value =
            serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(context["paragraph"], "原文");
        assert_eq!(context["comments"].as_array().unwrap().len(), 2);
        app.locator(Selector::id("live_editor")).assert_text("原文");
        release.send(()).unwrap();
        app.locator(Selector::id("live_editor"))
            .wait_text("处理后的段落");
        app.locator(Selector::id("transcript"))
            .wait_text("你\n改善原文\n\n你\n按新增要求修改\n\nAgent\n已按最新评论修改");
    })
    .unwrap();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
