//! Shared document type scale (logical points, not Retina image pixels).
//! Native Markdown scales headings relative to its base size. Each document
//! renderer receives one source block, so compensate locally without forking
//! Makepad or changing fonts for unrelated UI controls.
use crate::markdown::DocMarkdown as Markdown;

pub const BODY: f32 = 14.0;
const HEADING_SCALE: f32 = 2.0;
const NATIVE_FACTORS: [f32; 6] = [1.0, 0.75, 0.58, 0.5, 0.42, 0.33];
pub const HEADINGS: [f32; 6] = [28.0, 22.0, 18.0, 16.0, 14.0, 14.0];

fn heading_level(source: &str) -> Option<usize> {
    let first = source.lines().next()?.trim_start();
    let hashes = first.bytes().take_while(|c| *c == b'#').count();
    if (1..=6).contains(&hashes)
        && first
            .as_bytes()
            .get(hashes)
            .is_some_and(|c| c.is_ascii_whitespace())
    {
        return Some(hashes - 1);
    }
    // Setext headings, handled by the same parser as ATX headings.
    let second = source.lines().nth(1)?.trim();
    if first.is_empty() || second.is_empty() {
        return None;
    }
    if second.bytes().all(|c| c == b'=') {
        Some(0)
    } else if second.bytes().all(|c| c == b'-') {
        Some(1)
    } else {
        None
    }
}
fn base_size(source: &str) -> f32 {
    heading_level(source)
        .map(|i| HEADINGS[i] / (HEADING_SCALE * NATIVE_FACTORS[i]))
        .unwrap_or(BODY)
}
pub fn apply(markdown: &mut Markdown, source: &str) {
    // Document templates set heading_base_scale=2.0; do not use on message UI.
    markdown.font_size = base_size(source);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_scale_has_no_headings_smaller_than_body() {
        assert_eq!(base_size("正文 **粗体**"), BODY);
        for i in 0..6 {
            let s = format!("{} 标题", "#".repeat(i + 1));
            assert!((base_size(&s) * HEADING_SCALE * NATIVE_FACTORS[i] - HEADINGS[i]).abs() < 0.01);
            assert!(HEADINGS[i] >= BODY);
        }
        assert_eq!(base_size("标题\n===") * HEADING_SCALE, HEADINGS[0]);
        assert_eq!(base_size("标题\n---") * HEADING_SCALE * 0.75, HEADINGS[1]);
    }
}
