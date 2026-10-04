mod support;
use makepad_test::Selector;
#[test]
fn reading_and_live_preview_share_document_type_scale() {
    support::library::run("type_scale", |app| {
        support::library::fill_document(
            &app,
            "# 一级标题\n\n## 二级标题\n\n### 三级标题\n\n普通正文，用更适合长时间阅读的字号。\n",
        );
        app.locator(Selector::id("mode_edit")).click();
        support::wait_for_ink(&app, "preview");
        let snapshot = app.widget_snapshot();
        let mut reading: Vec<_> = snapshot.iter().filter(|w| w.id == "markdown").collect();
        reading.sort_by_key(|w| w.y);
        assert_eq!(reading.len(), 3);
        let heights: Vec<_> = reading.iter().map(|w| w.height).collect();
        assert!(
            heights[0] > heights[1] && heights[1] > heights[2],
            "heading/body hierarchy: {heights:?}"
        );
        println!("document type scale: {}", app.screenshot().display());
        app.locator(Selector::id("mode_read")).click();
        support::wait_for_ink(&app, "live_editor");
        let snapshot = app.widget_snapshot();
        let mut live: Vec<_> = snapshot.iter().filter(|w| w.id == "rendered").collect();
        live.sort_by_key(|w| w.y);
        assert_eq!(live.len(), 3);
        for (live, height) in live.iter().zip(heights) {
            assert!(
                (live.height - height).abs() < 2,
                "reading/live should match type scale"
            );
        }
    });
}
