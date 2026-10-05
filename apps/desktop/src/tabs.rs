//! Pure tab-close planning; no UI, storage, or provider authority.
#[derive(Clone, Copy)]
pub enum Close {
    Current,
    Left,
    Others,
    All,
}
pub fn remaining(tabs: &[String], target: &str, command: Close) -> Option<Vec<String>> {
    let index = tabs.iter().position(|t| t == target)?;
    Some(
        tabs.iter()
            .enumerate()
            .filter(|(i, _)| match command {
                Close::Current => *i != index,
                Close::Left => *i >= index,
                Close::Others => *i == index,
                Close::All => false,
            })
            .map(|(_, t)| t.clone())
            .collect(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_scopes_preserve_order_and_do_not_delete_documents() {
        let tabs = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(remaining(&tabs, "b", Close::Current).unwrap(), ["a", "c"]);
        assert_eq!(remaining(&tabs, "b", Close::Left).unwrap(), ["b", "c"]);
        assert_eq!(remaining(&tabs, "b", Close::Others).unwrap(), ["b"]);
        assert!(remaining(&tabs, "b", Close::All).unwrap().is_empty());
        assert!(remaining(&tabs, "missing", Close::All).is_none());
        assert!(
            remaining(&["a".into()], "a", Close::Current)
                .unwrap()
                .is_empty()
        );
    }
}
