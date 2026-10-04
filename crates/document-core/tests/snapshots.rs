use document_core::{EditError, ProposalStatus, Workbench, WorkbenchSnapshot};

fn restore(json: serde_json::Value) -> Result<Workbench, EditError> {
    let snapshot: WorkbenchSnapshot = serde_json::from_value(json).unwrap();
    Workbench::restore(snapshot)
}

#[test]
fn pending_restores_without_writing_and_needs_new_accept() {
    let mut w = Workbench::new("中文原文").unwrap();
    let annotation = w.annotate(0..12, "修改").unwrap();
    let proposal = w.propose(annotation, "中文提案").unwrap();
    let mut restored = restore(serde_json::to_value(w.snapshot()).unwrap()).unwrap();
    assert_eq!(restored.text(), "中文原文");
    assert_eq!(
        restored.proposal(proposal).unwrap().status(),
        ProposalStatus::Pending
    );
    restored.accept(proposal).unwrap();
    let mut restored = Workbench::restore(restored.snapshot()).unwrap();
    assert_eq!(restored.text(), "中文提案");
    assert_eq!(
        restored.accept(proposal),
        Err(EditError::ProposalAlreadyResolved)
    );
    restored.undo().unwrap();
    assert_eq!(restored.text(), "中文原文");
    assert_eq!(
        restored.accept(proposal),
        Err(EditError::ProposalAlreadyResolved)
    );
}

#[test]
fn stale_pending_restores_but_cannot_write() {
    let mut w = Workbench::new("abc").unwrap();
    let a = w.annotate(0..3, "修改").unwrap();
    let p = w.propose(a, "xyz").unwrap();
    w.edit("new").unwrap();
    let mut restored = Workbench::restore(w.snapshot()).unwrap();
    assert_eq!(restored.accept(p), Err(EditError::Conflict));
    assert_eq!(restored.text(), "new");
}

#[test]
fn malformed_snapshot_fails_before_workbench_exists() {
    let mut w = Workbench::new("中文").unwrap();
    let a = w.annotate(0..6, "修改").unwrap();
    w.propose(a, "new").unwrap();
    let baseline = serde_json::to_value(w.snapshot()).unwrap();
    let mut v = baseline.clone();
    v["proposals"][0]["annotation"] = 999.into();
    assert!(matches!(restore(v), Err(EditError::InvalidSnapshot)));
    let mut v = baseline.clone();
    v["annotations"][0]["range"]["start"] = 1.into();
    assert!(matches!(restore(v), Err(EditError::InvalidSnapshot)));
    let mut v = baseline.clone();
    v["annotations"][0]["original"] = "伪造".into();
    assert!(matches!(restore(v), Err(EditError::InvalidSnapshot)));
    let mut v = baseline.clone();
    v["revision"] = u64::MAX.into();
    assert!(matches!(restore(v), Err(EditError::InvalidSnapshot)));
    let mut v = baseline;
    v["history"] = serde_json::json!(["fake undo"]);
    assert!(matches!(restore(v), Err(EditError::InvalidSnapshot)));
}

#[test]
fn review_record_quota_is_enforced() {
    let mut w = Workbench::new("a").unwrap();
    for _ in 0..256 {
        w.annotate(0..1, "修改").unwrap();
    }
    assert_eq!(w.annotate(0..1, "修改"), Err(EditError::StateLimit));
    for _ in 0..256 {
        w.propose(0, "b").unwrap();
    }
    assert_eq!(w.propose(0, "b"), Err(EditError::StateLimit));
    let restored = Workbench::restore(w.snapshot()).unwrap();
    assert_eq!(restored.annotation_count(), 256);
}
