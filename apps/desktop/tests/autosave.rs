mod support;
use makepad_test::{KeyCode, KeyModifiers, Selector};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn document(root: &Path) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == "md"))
        .unwrap()
}
fn pause_until(start: Instant, elapsed: Duration) {
    if let Some(remaining) = elapsed.checked_sub(start.elapsed()) {
        std::thread::sleep(remaining);
    }
}

#[test]
fn spaces_remain_visible_and_keep_caret_during_save_and_undo() {
    support::library::run("body_spaces", |app| {
        support::library::fill_document(&app, "# 空格\n\n正文");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line")).wait_text("正文");
        app.press_key(KeyCode::End);
        for text in ["正文 ", "正文  ", "正文   "] {
            app.type_text(" ");
            app.locator(Selector::id("active_line")).wait_text(text);
            app.locator(Selector::id("live_editor"))
                .wait_text(format!("# 空格\n\n{text}"));
        }
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        app.locator(Selector::id("active_line"))
            .assert_text("正文   ");
        // Cursor movement starts a new native undo group after the spaces.
        app.press_key(KeyCode::Home);
        app.press_key(KeyCode::End);
        app.type_text("ff");
        app.locator(Selector::id("active_line"))
            .wait_text("正文   ff");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 空格\n\n正文   ff");
        app.press_key_with_modifiers(
            KeyCode::KeyZ,
            KeyModifiers {
                logo: true,
                ..Default::default()
            },
        );
        app.locator(Selector::id("active_line"))
            .wait_text("正文   ");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 空格\n\n正文   ");
        app.locator(Selector::id("comment_input")).click();
        app.locator(Selector::id("rendered")).click();
        app.locator(Selector::id("active_line"))
            .wait_text("正文   ");
        app.press_key(KeyCode::End);
        app.type_text("字");
        app.locator(Selector::id("active_line"))
            .wait_text("正文   字");
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        assert_eq!(
            fs::read_to_string(document(&support::library::root("body_spaces"))).unwrap(),
            "# 空格\n\n正文   字"
        );
    });
}

#[test]
fn autosave_waits_three_seconds_and_each_edit_postpones_disk_write() {
    support::library::run("save_debounce", |app| {
        let path = document(&support::library::root("save_debounce"));
        let baseline = fs::read_to_string(&path).unwrap();
        support::library::fill_document(&app, "# 保存\n\nfirst");
        app.locator(Selector::id("save_label"))
            .wait_text("等待自动保存…");
        let first = Instant::now();
        pause_until(first, Duration::from_millis(700));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            baseline,
            "must not save each keystroke"
        );
        app.press_key(KeyCode::End);
        app.type_text(" next");
        app.locator(Selector::id("live_editor"))
            .wait_text("# 保存\n\nfirst next");
        let last = Instant::now();
        pause_until(last, Duration::from_millis(1800));
        // Never write the new snapshot early. The old-deadline restart itself
        // is exact in autosave unit tests; remote input latency is not a clock.
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            baseline,
            "new input must postpone old deadline"
        );
        app.locator(Selector::id("save_label"))
            .assert_text("等待自动保存…");
        // Layout-only actions must not restart the save clock.
        app.locator(Selector::id("mode_edit")).click();
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        assert!(
            last.elapsed() >= Duration::from_millis(2800),
            "saved before three-second quiet period"
        );
        assert!(
            last.elapsed() < Duration::from_millis(4300),
            "non-edit action postponed the save"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "# 保存\n\nfirst next");
    });
}

#[test]
fn disabling_autosave_cancels_dirty_deadline_and_reenabling_restarts_it() {
    support::library::run("save_toggle", |app| {
        let path = document(&support::library::root("save_toggle"));
        let baseline = fs::read_to_string(&path).unwrap();
        support::library::fill_document(&app, "# 开关\n\n尚未保存");
        support::open_settings(&app);
        app.locator(Selector::id("pref_nav_editor")).click();
        app.locator(Selector::id("pref_auto_save")).click();
        std::thread::sleep(Duration::from_millis(3400));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            baseline,
            "disabled autosave must cancel timer"
        );
        app.locator(Selector::id("pref_auto_save")).click();
        std::thread::sleep(Duration::from_millis(1600));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            baseline,
            "enabling starts a new three-second deadline"
        );
        app.locator(Selector::id("preferences_done")).click();
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
        assert_eq!(fs::read_to_string(&path).unwrap(), "# 开关\n\n尚未保存");
    });
}
#[test]
fn switching_documents_flushes_without_waiting_for_debounce() {
    support::library::run("save_flush_switch", |app| {
        let path = document(&support::library::root("save_flush_switch"));
        support::library::fill_document(&app, "# 切换\n\n带尾空格 ");
        app.locator(Selector::id("save_label"))
            .wait_text("等待自动保存…");
        app.locator(Selector::id("new_document")).click();
        app.locator(Selector::id("live_editor"))
            .wait_text("# 未命名文档\n\n");
        assert_eq!(fs::read_to_string(path).unwrap(), "# 切换\n\n带尾空格 ");
    });
}
