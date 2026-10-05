use document_core::{EditError, Workbench};
#[test]
fn host_metadata_roundtrips_without_changing_revision_or_authority() {
    let mut w = Workbench::new("选中词，段落其它内容").unwrap();
    let id = w.comment(0..9, "解释").unwrap();
    let request = w.comment_request(id).unwrap();
    w.stamp_last_message(id, "你", 1791130000).unwrap();
    assert_eq!(w.revision(), 0);
    assert_eq!(w.thread(id).unwrap().range(), 0..9);
    assert_eq!(
        w.stamp_last_message(id, "其他作者", 1791130001),
        Err(EditError::Conflict)
    );
    let restored = Workbench::restore(w.snapshot()).unwrap();
    assert_eq!(
        restored.thread(id).unwrap().messages()[0].author.as_deref(),
        Some("你")
    );
    assert_eq!(
        restored.thread(id).unwrap().messages()[0].created_at,
        Some(1791130000)
    );
    // Metadata changes do not grant permission or change an in-flight request:
    // a previously captured request must be explicitly regenerated.
    assert_eq!(
        w.apply_comment_result(&request, "变更", "结果"),
        Err(EditError::Conflict)
    );
    let mut legacy = serde_json::to_value(w.snapshot()).unwrap();
    let message = &mut legacy["threads"][0]["messages"][0];
    message.as_object_mut().unwrap().remove("author");
    message.as_object_mut().unwrap().remove("created_at");
    let old = Workbench::restore(serde_json::from_value(legacy.clone()).unwrap()).unwrap();
    assert_eq!(old.thread(0).unwrap().messages()[0].created_at, None);
    legacy["threads"][0]["messages"][0]["created_at"] = serde_json::json!(-1);
    assert!(Workbench::restore(serde_json::from_value(legacy).unwrap()).is_err());
}
