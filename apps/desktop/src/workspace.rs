//! App-owned layout policy. Sizes are Makepad logical points.
pub const ARTICLE_WIDTH: f64 = 760.0;
pub fn sidebars(width: f64, navigation_requested: bool, comments_requested: bool) -> (bool, bool) {
    let comments = comments_requested && width >= 760.0;
    let navigation = navigation_requested && width >= 760.0 && (!comments || width >= 1100.0);
    (navigation, comments)
}
/// Only the top/bottom 20 logical points enable selection auto-scroll.
/// Positive moves toward the document start, negative toward the end.
pub fn selection_edge_scroll(top: f64, height: f64, pointer_y: f64) -> f64 {
    if pointer_y < top + 20.0 {
        (2.0 + (top + 20.0 - pointer_y) * 0.25).clamp(2.0, 16.0)
    } else if pointer_y > top + height - 20.0 {
        -(2.0 + (pointer_y - (top + height - 20.0)) * 0.25).clamp(2.0, 16.0)
    } else {
        0.0
    }
}
/// Optional desktop startup sizing, also useful for reproducible visual QA.
pub fn window_size(argument: &str) -> Option<(f64, f64)> {
    let (width, height) = argument.strip_prefix("--window-size=")?.split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    ((600..=4096).contains(&width) && (600..=2160).contains(&height))
        .then_some((width as f64, height as f64))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_windows_keep_document_space_without_changing_user_intent() {
        assert_eq!(sidebars(1400.0, true, true), (true, true));
        assert_eq!(sidebars(900.0, true, true), (false, true));
        assert_eq!(sidebars(900.0, true, false), (true, false));
        assert_eq!(sidebars(700.0, true, true), (false, false));
        assert_eq!(sidebars(1400.0, false, false), (false, false));
        assert_eq!(sidebars(1100.0, true, true), (true, true));
    }
    #[test]
    fn selection_scroll_only_at_visible_edges() {
        for y in [120.0, 200.0, 380.0, 480.0] {
            assert_eq!(selection_edge_scroll(100.0, 400.0, y), 0.0);
        }
        assert!(selection_edge_scroll(100.0, 400.0, 101.0) > 0.0);
        assert!(selection_edge_scroll(100.0, 400.0, 499.0) < 0.0);
        assert_eq!(selection_edge_scroll(100.0, 400.0, 1000.0), -16.0);
    }
    #[test]
    fn startup_size_is_explicit_and_bounded() {
        assert_eq!(window_size("--window-size=900x800"), Some((900.0, 800.0)));
        for input in [
            "900x800",
            "--window-size=0x800",
            "--window-size=nanx800",
            "--window-size=99999x800",
        ] {
            assert_eq!(window_size(input), None);
        }
    }
}
