use document_core::{EditError, Workbench};

#[test]
fn partial_unicode_anchor_limits_model_write_and_undo_invalidates_it() {
    let source = "前面 **选中词** 后面 [重复](https://example.com/重复)";
    let start = source.find("选中词").unwrap();
    let range = start..start + "选中词".len();
    let mut workbench = Workbench::new(source).unwrap();
    let id = workbench
        .comment_with_ai(range.clone(), "只修改选中文字", true)
        .unwrap();
    let request = workbench.comment_request(id).unwrap();
    assert_eq!(request.original, "选中词");
    workbench
        .apply_comment_result(&request, "替换词", "已修改选区")
        .unwrap();
    assert_eq!(
        workbench.text(),
        "前面 **替换词** 后面 [重复](https://example.com/重复)"
    );
    assert_eq!(workbench.thread(id).unwrap().range(), range);
    let next = workbench.comment_request(id).unwrap();
    workbench.undo().unwrap();
    assert_eq!(workbench.text(), source);
    assert_eq!(
        workbench.apply_comment_result(&next, "旧结果", "迟到"),
        Err(EditError::Conflict)
    );
    assert_eq!(workbench.text(), source);
}

#[test]
fn precise_anchor_restore_shift_reject_and_rebind_do_not_enlarge_scope() {
    let source = "重复词 第一个；重复词 第二个";
    let start = source.rfind("重复词").unwrap();
    let mut w = Workbench::new(source).unwrap();
    let id = w.comment(start..start + 9, "仅第二个").unwrap();
    let old = w.comment_request(id).unwrap();
    w.edit(format!("前缀 {source}")).unwrap();
    assert_eq!(w.thread(id).unwrap().original(), "重复词");
    assert_eq!(w.thread(id).unwrap().range(), start + 7..start + 16);
    assert_eq!(
        w.apply_comment_result(&old, "错误", "迟到"),
        Err(EditError::Conflict)
    );
    let mut restored = Workbench::restore(w.snapshot()).unwrap();
    restored.resolve_thread(id).unwrap();
    assert_eq!(
        restored.comment_request(id),
        Err(EditError::ProposalAlreadyResolved)
    );
    restored.reopen_thread(id).unwrap();
    restored.rebind_thread(id, 7..16).unwrap();
    assert_eq!(restored.thread(id).unwrap().original(), "重复词");
    assert_eq!(restored.thread(id).unwrap().range(), 7..16);
    assert_eq!(restored.text(), format!("前缀 {source}"));
}
