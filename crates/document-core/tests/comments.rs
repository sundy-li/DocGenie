use document_core::{EditError, Speaker, Workbench};

#[test]
fn multi_round_updates_anchor_and_transcript_with_undo() {
    let mut w = Workbench::new("前\n\n原段落\n\n后").unwrap();
    let id = w.comment(5..14, "补充一个例子").unwrap();
    let request = w.comment_request(id).unwrap();
    w.apply_comment_result(&request, "第一轮", "已补充例子")
        .unwrap();
    assert_eq!(w.text(), "前\n\n第一轮\n\n后");
    assert_eq!(w.thread(id).unwrap().messages()[1].speaker, Speaker::Agent);
    w.reply(id, "再缩短一点").unwrap();
    let next = w.comment_request(id).unwrap();
    w.apply_comment_result(&next, "精简", "已精简").unwrap();
    assert_eq!(w.text(), "前\n\n精简\n\n后");
    let mut restored = Workbench::restore(w.snapshot()).unwrap();
    assert_eq!(restored.thread(id).unwrap().messages().len(), 4);
    restored.undo().unwrap();
    assert_eq!(restored.text(), "前\n\n第一轮\n\n后");
    assert_eq!(restored.comment_request(id), Err(EditError::Conflict));
}
#[test]
fn newer_reply_invalidates_inflight_result() {
    let mut w = Workbench::new("原段落").unwrap();
    let id = w.comment(0..9, "补充").unwrap();
    let old = w.comment_request(id).unwrap();
    w.reply(id, "不要改变事实").unwrap();
    assert_eq!(
        w.apply_comment_result(&old, "旧结果", "已修改"),
        Err(EditError::Conflict)
    );
    assert_eq!(w.text(), "原段落");
    w.apply_comment_result(&w.comment_request(id).unwrap(), "新结果", "遵循新要求")
        .unwrap();
}
#[test]
fn manual_edit_and_resolve_block_automatic_write() {
    let mut w = Workbench::new("原段落").unwrap();
    let id = w.comment(0..9, "修改").unwrap();
    let request = w.comment_request(id).unwrap();
    w.edit("人工内容").unwrap();
    assert_eq!(
        w.apply_comment_result(&request, "旧结果", "修改"),
        Err(EditError::Conflict)
    );
    assert_eq!(w.text(), "人工内容");
    let mut w = Workbench::new("原段落").unwrap();
    let id = w.comment(0..9, "修改").unwrap();
    let request = w.comment_request(id).unwrap();
    w.resolve_thread(id).unwrap();
    assert_eq!(
        w.apply_comment_result(&request, "旧结果", "修改"),
        Err(EditError::ProposalAlreadyResolved)
    );
}
#[test]
fn empty_output_is_deletion_and_replay_fails() {
    let mut w = Workbench::new("原段落").unwrap();
    let id = w.comment(0..9, "删除").unwrap();
    let request = w.comment_request(id).unwrap();
    w.apply_comment_result(&request, "", "已删除").unwrap();
    assert_eq!(w.text(), "");
    assert!(Workbench::restore(w.snapshot()).is_ok());
    assert_eq!(
        w.apply_comment_result(&request, "重复", "修改"),
        Err(EditError::Conflict)
    );
}
#[test]
fn malformed_result_is_atomic() {
    let mut w = Workbench::new("原段落").unwrap();
    let id = w.comment(0..9, "修改").unwrap();
    let request = w.comment_request(id).unwrap();
    assert_eq!(
        w.apply_comment_result(&request, "结果", ""),
        Err(EditError::EmptyInstruction)
    );
    assert_eq!(w.text(), "原段落");
    assert_eq!(w.thread(id).unwrap().messages().len(), 1);
}
#[test]
fn ask_ai_flag_persists_and_defaults_off() {
    let mut w = Workbench::new("原段落").unwrap();
    let plain = w.comment(0..3, "普通评论").unwrap();
    let ai = w.comment_with_ai(0..9, "问 AI 的评论", true).unwrap();
    assert!(!w.thread(plain).unwrap().ask_ai());
    assert!(w.thread(ai).unwrap().ask_ai());
    let restored = Workbench::restore(w.snapshot()).unwrap();
    assert!(!restored.thread(plain).unwrap().ask_ai());
    assert!(restored.thread(ai).unwrap().ask_ai());
    // Threads persisted before the flag existed deserialize with ask_ai=false.
    let legacy = serde_json::json!({
        "range": {"start": 0, "end": 3},
        "revision": 0,
        "original": "原",
        "round": 1,
        "messages": [{"speaker": "User", "text": "旧评论"}],
        "resolved": false
    });
    let thread: document_core::CommentThread = serde_json::from_value(legacy).unwrap();
    assert!(!thread.ask_ai());
}
