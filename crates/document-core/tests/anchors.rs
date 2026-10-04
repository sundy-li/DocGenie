use document_core::{EditError, Workbench};
#[test]
fn unrelated_edit_moves_anchor_but_old_request_still_fails() {
    let mut w = Workbench::new("前段\n\n目标 **段落**\n\n尾段").unwrap();
    let start = w.text().find("目标").unwrap();
    let end = w.text().find("\n\n尾段").unwrap();
    let id = w.comment(start..end, "修改").unwrap();
    let old = w.comment_request(id).unwrap();
    w.edit("新前段落\n\n目标 **段落**\n\n尾段").unwrap();
    assert_eq!(w.comment_request(id).unwrap().original, "目标 **段落**");
    assert_eq!(
        w.apply_comment_result(&old, "结果", "已修改"),
        Err(EditError::Conflict)
    );
    let fresh = w.comment_request(id).unwrap();
    w.apply_comment_result(&fresh, "结果", "已修改").unwrap();
    assert_eq!(w.text(), "新前段落\n\n结果\n\n尾段");
    assert!(Workbench::restore(w.snapshot()).is_ok());
}
#[test]
fn overlapping_change_requires_explicit_rebind() {
    let mut w = Workbench::new("原文").unwrap();
    let id = w.comment(0..6, "修改").unwrap();
    let old = w.comment_request(id).unwrap();
    w.edit("人工新段落").unwrap();
    assert_eq!(w.comment_request(id), Err(EditError::Conflict));
    w.rebind_thread(id, 0..15).unwrap();
    assert_eq!(
        w.apply_comment_result(&old, "旧结果", "修改"),
        Err(EditError::Conflict)
    );
    w.apply_comment_result(
        &w.comment_request(id).unwrap(),
        "最新结果",
        "按最新原文修改",
    )
    .unwrap();
    assert_eq!(w.text(), "最新结果");
}
#[test]
fn resolved_thread_can_reopen_without_replaying_old_result() {
    let mut w = Workbench::new("原文").unwrap();
    let id = w.comment(0..6, "修改").unwrap();
    w.resolve_thread(id).unwrap();
    assert!(w.comment_request(id).is_err());
    w.reopen_thread(id).unwrap();
    w.reply(id, "重新处理").unwrap();
    assert_eq!(w.comment_request(id).unwrap().messages.len(), 2);
}
