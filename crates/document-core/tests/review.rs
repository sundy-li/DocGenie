use document_core::{EditError, MAX_DOCUMENT_BYTES, ProposalStatus, Workbench};

fn proposal(w: &mut Workbench, range: std::ops::Range<usize>, text: &str) -> usize {
    let annotation = w.annotate(range, "保留事实，缩短表达").unwrap();
    w.propose(annotation, text).unwrap()
}

#[test]
fn chinese_selection_is_byte_based_and_preserves_surroundings() {
    let mut w = Workbench::new("开头：原始段落。结尾").unwrap();
    let id = proposal(&mut w, 9..21, "新段落");
    assert_eq!(w.text(), "开头：原始段落。结尾");
    w.accept(id).unwrap();
    assert_eq!(w.text(), "开头：新段落。结尾");
    assert_eq!(w.proposal(id).unwrap().status(), ProposalStatus::Applied);
}

#[test]
fn invalid_utf8_boundaries_empty_reversed_and_oob_are_rejected() {
    let mut w = Workbench::new("你好👋").unwrap();
    for range in [
        1..3,
        0..2,
        0..99,
        3..3,
        std::ops::Range { start: 6, end: 3 },
    ] {
        assert_eq!(w.annotate(range, "修改"), Err(EditError::InvalidSelection));
    }
    assert_eq!(w.annotate(0..3, " \n"), Err(EditError::EmptyInstruction));
}

#[test]
fn changed_document_cannot_be_overwritten_by_old_proposal() {
    let mut w = Workbench::new("原文").unwrap();
    let id = proposal(&mut w, 0..6, "建议");
    w.edit("人工修改").unwrap();
    assert_eq!(w.accept(id), Err(EditError::Conflict));
    assert_eq!(w.text(), "人工修改");
    assert_eq!(w.proposal(id).unwrap().status(), ProposalStatus::Conflicted);
    assert_eq!(w.accept(id), Err(EditError::ProposalAlreadyResolved));
}

#[test]
fn rejected_and_applied_proposals_are_not_replayable() {
    let mut w = Workbench::new("abc").unwrap();
    let id = proposal(&mut w, 0..3, "xyz");
    w.reject(id).unwrap();
    assert_eq!(w.text(), "abc");
    assert_eq!(w.accept(id), Err(EditError::ProposalAlreadyResolved));
    assert_eq!(w.reject(id), Err(EditError::ProposalAlreadyResolved));
    let id = proposal(&mut w, 0..3, "xyz");
    w.accept(id).unwrap();
    assert_eq!(w.accept(id), Err(EditError::ProposalAlreadyResolved));
}

#[test]
fn undo_is_a_new_revision_and_does_not_restore_approval() {
    let mut w = Workbench::new("abc").unwrap();
    let first = proposal(&mut w, 0..3, "xyz");
    let second = proposal(&mut w, 0..3, "other");
    w.accept(first).unwrap();
    w.undo().unwrap();
    assert_eq!(w.text(), "abc");
    assert_eq!(w.revision(), 2);
    assert_eq!(w.accept(first), Err(EditError::ProposalAlreadyResolved));
    assert_eq!(w.accept(second), Err(EditError::Conflict));
}

#[test]
fn delayed_generation_checks_annotation_revision() {
    let mut w = Workbench::new("abc").unwrap();
    let annotation = w.annotate(0..3, "修改").unwrap();
    w.edit("abcd").unwrap();
    assert_eq!(w.propose(annotation, "xyz"), Err(EditError::Conflict));
}

#[test]
fn empty_replacement_deletes_only_target() {
    let mut w = Workbench::new("前删除后").unwrap();
    let id = proposal(&mut w, 3..9, "");
    w.accept(id).unwrap();
    assert_eq!(w.text(), "前后");
    w.undo().unwrap();
    assert_eq!(w.text(), "前删除后");
}

#[test]
fn unknown_ids_return_errors_and_noop_edit_preserves_revision() {
    let mut w = Workbench::new("abc").unwrap();
    assert_eq!(w.accept(42), Err(EditError::UnknownProposal));
    assert_eq!(w.reject(42), Err(EditError::UnknownProposal));
    assert_eq!(w.propose(42, "xyz"), Err(EditError::UnknownAnnotation));
    assert_eq!(w.undo(), Err(EditError::NothingToUndo));
    w.edit("abc").unwrap();
    assert_eq!(w.revision(), 0);
}

#[test]
fn size_limit_failure_is_atomic() {
    assert!(matches!(
        Workbench::new("x".repeat(MAX_DOCUMENT_BYTES + 1)),
        Err(EditError::DocumentTooLarge)
    ));
    let mut w = Workbench::new("ab").unwrap();
    let id = proposal(&mut w, 0..1, &"x".repeat(MAX_DOCUMENT_BYTES));
    assert_eq!(w.accept(id), Err(EditError::DocumentTooLarge));
    assert_eq!(w.text(), "ab");
    assert_eq!(w.revision(), 0);
    assert_eq!(w.proposal(id).unwrap().status(), ProposalStatus::Pending);
    assert_eq!(
        w.edit("x".repeat(MAX_DOCUMENT_BYTES + 1)),
        Err(EditError::DocumentTooLarge)
    );
}

#[test]
fn undo_history_is_bounded() {
    let mut w = Workbench::new("0").unwrap();
    for i in 1..=110 {
        w.edit(i.to_string()).unwrap();
    }
    for _ in 0..100 {
        w.undo().unwrap();
    }
    assert_eq!(w.text(), "10");
    assert_eq!(w.undo(), Err(EditError::NothingToUndo));
}

#[test]
fn set_text_replaces_text_and_advances_revision() {
    let mut wb = Workbench::new("").unwrap();
    let r0 = wb.revision();
    wb.set_text("新的标题".into()).unwrap();
    assert_eq!(wb.text(), "新的标题");
    assert_eq!(wb.revision(), r0 + 1);
    // Undo restores prior text but revision keeps growing.
    wb.undo().unwrap();
    assert_eq!(wb.text(), "");
    assert!(wb.revision() > r0 + 1);
}

#[test]
fn set_text_rejects_oversize_input() {
    let mut wb = Workbench::new("").unwrap();
    let huge = "x".repeat(MAX_DOCUMENT_BYTES + 1);
    assert!(matches!(
        wb.set_text(huge),
        Err(EditError::DocumentTooLarge)
    ));
}
