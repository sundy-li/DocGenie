//! Lossless visible-text edits. Markdown is canonical; presentation is not.
//! No UI, IO, or provider dependency. Never strip syntax and save plain text.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub link: bool,
    pub heading: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub visible: Range<usize>,
    pub source: Range<usize>,
    pub style: Style,
}
#[derive(Clone, Debug, Default)]
pub struct Projection {
    pub text: String,
    pub runs: Vec<Run>,
}
impl Projection {
    /// The active heading alone exposes its ATX prefix. Inline syntax never
    /// becomes an editable source field. Table structure is deliberately not
    /// flattened into editable text; table cells get separate projections.
    pub fn new(source: &str, active_heading: bool) -> Self {
        let mut result = Self::default();
        let mut style = Style::default();
        let mut stack = Vec::new();
        let mut image = 0;
        for (event, range) in Parser::new_ext(
            source,
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
        )
        .into_offset_iter()
        {
            match event {
                Event::Start(tag) => {
                    if matches!(
                        tag,
                        Tag::Paragraph
                            | Tag::Heading { .. }
                            | Tag::Item
                            | Tag::TableRow
                            | Tag::CodeBlock(_)
                    ) && !result.text.is_empty()
                        && !result.text.ends_with('\n')
                    {
                        result.text.push('\n');
                    }
                    stack.push(style.clone());
                    if matches!(tag, Tag::Paragraph) {
                        let line_start = source[..range.start].rfind('\n').map_or(0, |i| i + 1);
                        let start = if source[line_start..range.start]
                            .bytes()
                            .all(|b| matches!(b, b' ' | b'\t'))
                        {
                            line_start
                        } else {
                            range.start
                        };
                        let leading = source[range.clone()]
                            .bytes()
                            .take_while(|b| matches!(b, b' ' | b'\t'))
                            .count();
                        if start < range.start + leading {
                            result.push(source, start..range.start + leading, style.clone());
                        }
                    }
                    match tag {
                        Tag::Strong => style.bold = true,
                        Tag::Emphasis => style.italic = true,
                        Tag::Link { .. } => style.link = true,
                        Tag::CodeBlock(_) => style.code = true,
                        Tag::Image { .. } => image += 1,
                        Tag::Heading { level, .. } => {
                            style.heading = Some(level as usize - 1);
                            style.bold = true;
                            if active_heading {
                                let prefix = source[range.start..]
                                    .bytes()
                                    .take_while(|b| *b == b'#')
                                    .count();
                                // CommonMark recognizes a marker-only `#` or
                                // `##` as an empty heading, before a separator
                                // is typed. Keep it editable and visible too:
                                // hiding it here would overwrite the native
                                // buffer on the next draw and lose the caret.
                                let after = range.start + prefix;
                                let whitespace = source[after..range.end]
                                    .bytes()
                                    .take_while(|b| matches!(b, b' ' | b'\t'))
                                    .count();
                                if prefix > 0 {
                                    result.push(
                                        source,
                                        range.start..after + whitespace,
                                        style.clone(),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(tag) => {
                    // CommonMark drops paragraph/heading trailing whitespace.
                    // It must remain in the native editing buffer so redraw
                    // never eats a newly typed space or rewinds its caret.
                    if matches!(tag, TagEnd::Paragraph | TagEnd::Heading(_)) {
                        let end = source[..range.end].trim_end_matches(['\n', '\r']).len();
                        let start = source[..end].trim_end_matches([' ', '\t']).len();
                        let mapped_end = result.runs.last().map_or(range.start, |r| r.source.end);
                        let start = start.max(mapped_end).max(range.start);
                        if start < end {
                            result.push(source, start..end, style.clone());
                        }
                    }
                    if tag == TagEnd::Image {
                        image -= 1;
                    }
                    style = stack.pop().unwrap_or_default();
                }
                Event::Text(text) if image == 0 => {
                    // Decoded entities / backslash escapes retain their exact
                    // original bytes as one run; non-identity edits are rejected.
                    let start = result.text.len();
                    result.text.push_str(&text);
                    result.runs.push(Run {
                        visible: start..result.text.len(),
                        source: range,
                        style: style.clone(),
                    });
                }
                Event::Code(text) if image == 0 => {
                    let raw = &source[range.clone()];
                    let ticks = raw.bytes().take_while(|b| *b == b'`').count();
                    let content = range.start + ticks..range.end.saturating_sub(ticks);
                    let start = result.text.len();
                    result.text.push_str(&text);
                    let mut code = style.clone();
                    code.code = true;
                    result.runs.push(Run {
                        visible: start..result.text.len(),
                        source: content,
                        style: code,
                    });
                }
                Event::SoftBreak => result.push(source, range, style.clone()),
                _ => {}
            }
        }
        if result.runs.is_empty() && source.bytes().all(|b| matches!(b, b' ' | b'\t')) {
            result.push(source, 0..source.len(), Style::default());
        }
        if result.runs.is_empty() {
            // Empty source, and also paragraphs whose only content renders
            // nothing visible (a lone image, thematic break or HTML block):
            // without a run the caret lands in no run at all and every
            // keystroke is rejected. Map the caret to the paragraph start so
            // typing prepends text ahead of the invisible content.
            result.runs.push(Run {
                visible: 0..0,
                source: 0..0,
                style,
            });
        }
        // Adjacent parser fragments with identical style are one editable run
        // (in particular the active heading prefix and its title).
        let mut merged: Vec<Run> = Vec::new();
        for run in result.runs {
            if let Some(last) = merged.last_mut()
                && last.source.end == run.source.start
                && last.visible.end == run.visible.start
                && last.style == run.style
            {
                last.source.end = run.source.end;
                last.visible.end = run.visible.end;
            } else {
                merged.push(run);
            }
        }
        result.runs = merged;
        result
    }
    fn push(&mut self, source: &str, range: Range<usize>, style: Style) {
        let start = self.text.len();
        self.text.push_str(&source[range.clone()]);
        self.runs.push(Run {
            visible: start..self.text.len(),
            source: range,
            style,
        });
    }
    pub fn visible_range(&self, source: Range<usize>) -> Option<Range<usize>> {
        let first = self
            .runs
            .iter()
            .find(|r| r.source.end > source.start && r.source.start < source.end)?;
        let last = self
            .runs
            .iter()
            .rfind(|r| r.source.end > source.start && r.source.start < source.end)?;
        if first.source.len() != first.visible.len() || last.source.len() != last.visible.len() {
            return None;
        }
        Some(
            first.visible.start + source.start.saturating_sub(first.source.start)
                ..last.visible.start + source.end.min(last.source.end) - last.source.start,
        )
    }
    pub fn source_cursor(&self, visible: usize) -> Option<usize> {
        let run = self
            .runs
            .iter()
            .find(|r| r.visible.start <= visible && r.visible.end >= visible)?;
        Some(run.source.start + visible - run.visible.start)
    }
    pub fn visible_cursor(&self, source: usize) -> usize {
        self.runs
            .iter()
            .find(|r| r.source.start <= source && r.source.end >= source)
            .map(|r| r.visible.start + source - r.source.start)
            .unwrap_or_else(|| {
                self.runs
                    .iter()
                    .find(|r| r.source.start >= source)
                    .map_or(self.text.len(), |r| r.visible.start)
            })
    }
    pub fn source_range(&self, visible: Range<usize>) -> Option<Range<usize>> {
        if visible.start >= visible.end
            || !self.text.is_char_boundary(visible.start)
            || !self.text.is_char_boundary(visible.end)
        {
            return None;
        }
        let first = self.runs.iter().find(|r| r.visible.end > visible.start)?;
        let last = self.runs.iter().rfind(|r| r.visible.start < visible.end)?;
        if first.source.len() != first.visible.len() || last.source.len() != last.visible.len() {
            return None;
        }
        Some(
            first.source.start + visible.start.saturating_sub(first.visible.start)
                ..last.source.start + (visible.end.min(last.visible.end) - last.visible.start),
        )
    }
    /// Exact comment span, including paired formatting delimiters only when
    /// the entire formatted run is selected. Never include a containing block
    /// or link URL. Partial styled text stays a precise inner-text range.
    pub fn comment_range(&self, source: &str, visible: Range<usize>) -> Option<Range<usize>> {
        let mut mapped = self.source_range(visible.clone())?;
        for (event, range) in
            Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH).into_offset_iter()
        {
            if matches!(
                event,
                Event::Start(Tag::Strong | Tag::Emphasis | Tag::Strikethrough)
            ) {
                let mut runs = self
                    .runs
                    .iter()
                    .filter(|r| r.source.start >= range.start && r.source.end <= range.end);
                if let Some(first) = runs.next() {
                    let last = runs.next_back().unwrap_or(first);
                    if visible.start <= first.visible.start && visible.end >= last.visible.end {
                        mapped.start = mapped.start.min(range.start);
                        mapped.end = mapped.end.max(range.end);
                    }
                }
            }
        }
        Some(mapped)
    }
    /// Find the visible edit made by native typing / IME / undo.
    pub fn changed(&self, source: &str, next: &str) -> Option<String> {
        let prefix = self
            .text
            .chars()
            .zip(next.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let suffix = self.text[prefix..]
            .chars()
            .rev()
            .zip(next[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        self.edit(
            source,
            prefix..self.text.len() - suffix,
            &next[prefix..next.len() - suffix],
        )
    }
    /// Apply one displayed edit without touching any hidden delimiters or URL.
    /// Reject ambiguous decoded text, structural input and cross-run deletion
    /// rather than silently losing syntax. Wider editing requires a tree edit.
    pub fn edit(&self, source: &str, range: Range<usize>, replacement: &str) -> Option<String> {
        if range.start > range.end
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return None;
        }
        if let Some(run) = self
            .runs
            .iter()
            .find(|r| r.visible.start <= range.start && r.visible.end >= range.end)
        {
            if source.get(run.source.clone())? != self.text.get(run.visible.clone())? {
                return None;
            }
            let mut start = run.source.start + range.start - run.visible.start;
            let mut end = run.source.start + range.end - run.visible.start;
            if replacement.is_empty() && range == run.visible {
                for (event, container) in
                    Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH).into_offset_iter()
                {
                    if matches!(
                        event,
                        Event::Start(Tag::Strong | Tag::Emphasis | Tag::Link { .. })
                            | Event::Code(_)
                    ) && container.start <= start
                        && container.end >= end
                        && self
                            .runs
                            .iter()
                            .filter(|r| {
                                r.source.start >= container.start && r.source.end <= container.end
                            })
                            .count()
                            == 1
                    {
                        start = container.start;
                        end = container.end;
                    }
                }
            }
            let mut result = source.to_owned();
            result.replace_range(start..end, replacement);
            return Some(result);
        }
        // A cross-style edit removes only fully selected inline containers.
        // Partial containers would leave unbalanced syntax: reject those until
        // a full inline-tree split/merge is available, never flatten silently.
        let mut mapped = self.source_range(range.clone())?;
        for (event, container) in
            Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH).into_offset_iter()
        {
            if matches!(
                event,
                Event::Start(Tag::Strong | Tag::Emphasis | Tag::Link { .. } | Tag::Strikethrough)
            ) {
                let children: Vec<_> = self
                    .runs
                    .iter()
                    .filter(|r| r.source.start >= container.start && r.source.end <= container.end)
                    .collect();
                let Some(first) = children.first() else {
                    continue;
                };
                let last = children.last()?;
                let intersects = first.visible.start < range.end && last.visible.end > range.start;
                if intersects {
                    if range.start > first.visible.start || range.end < last.visible.end {
                        return None;
                    }
                    mapped.start = mapped.start.min(container.start);
                    mapped.end = mapped.end.max(container.end);
                }
            }
        }
        let mut result = source.to_owned();
        result.replace_range(mapped, replacement);
        Some(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visible_text_and_style_hide_syntax_and_urls() {
        let source = "普通 **中文粗体**，*斜体* 与 [链接](https://example.com/a) 和 `code`";
        let p = Projection::new(source, false);
        assert_eq!(p.text, "普通 中文粗体，斜体 与 链接 和 code");
        assert!(p.runs.iter().any(|r| r.style.bold));
        assert!(p.runs.iter().any(|r| r.style.italic));
        assert!(p.runs.iter().any(|r| r.style.link));
        assert!(p.runs.iter().any(|r| r.style.code));
    }
    #[test]
    fn incremental_heading_markers_and_separator_never_disappear() {
        let mut source = String::new();
        for next in ["#", "##", "## ", "## A", "## AB", "## ABC"] {
            source = Projection::new(&source, true)
                .changed(&source, next)
                .unwrap();
            let active = Projection::new(&source, true);
            assert_eq!(source, next);
            assert_eq!(active.text, next);
            assert_eq!(active.source_cursor(next.len()), Some(next.len()));
            assert_eq!(active.visible_cursor(next.len()), next.len());
        }
        assert_eq!(Projection::new(&source, false).text, "ABC");
        assert_eq!(
            Projection::new(&source, false).runs[0].style.heading,
            Some(1)
        );
        for next in ["## AB", "## A", "## ", "##", "#", ""] {
            source = Projection::new(&source, true)
                .changed(&source, next)
                .unwrap();
            assert_eq!(Projection::new(&source, true).text, next);
            assert_eq!(source, next);
        }
    }
    #[test]
    fn every_marker_only_heading_is_visible_while_active() {
        for level in 1..=6 {
            let marker = "#".repeat(level);
            for suffix in ["", " ", "  ", "\t", " ABC"] {
                let source = format!("{marker}{suffix}");
                let active = Projection::new(&source, true);
                assert_eq!(active.text, source);
                assert_eq!(active.source_cursor(source.len()), Some(source.len()));
            }
        }
        // Literal hashes inside paragraphs must keep their ordinary meaning.
        assert_eq!(Projection::new("正文 ##", true).text, "正文 ##");
        assert_eq!(Projection::new("##ABC", true).runs[0].style.heading, None);
    }
    #[test]
    fn heading_prefix_only_exposed_for_active_heading() {
        assert_eq!(Projection::new("## 蓝色标题", false).text, "蓝色标题");
        assert_eq!(Projection::new("## 蓝色标题", true).text, "## 蓝色标题");
        assert_eq!(Projection::new("正文 **文字**", true).text, "正文 文字");
    }
    #[test]
    fn edits_preserve_link_target_and_bold_delimiters() {
        let source = "**中文** [相同](https://example.com/相同)";
        let p = Projection::new(source, false);
        assert_eq!(
            p.edit(source, 0..6, "新文"),
            Some("**新文** [相同](https://example.com/相同)".into())
        );
        assert_eq!(
            p.edit(source, 7..13, "链接"),
            Some("**中文** [链接](https://example.com/相同)".into())
        );
        let mapped = p.source_range(7..13).unwrap();
        assert_eq!(&source[mapped], "相同");
    }
    #[test]
    fn cross_style_edits_remove_fully_selected_containers_and_reject_partial() {
        let source = "A **B** C";
        let p = Projection::new(source, false);
        assert_eq!(p.edit(source, 0..5, "D"), Some("D".into()));
        let partial = Projection::new("A **BC** D", false);
        assert_eq!(partial.edit("A **BC** D", 0..3, "X"), None);
        assert_eq!(
            Projection::new("&amp;", false).edit("&amp;", 0..1, "x"),
            None
        );
        assert_eq!(Projection::new("中文", false).edit("中文", 1..3, "x"), None);
    }
    #[test]
    fn deletion_and_typing_keep_source_valid() {
        for source in [
            "**删除**",
            "*删除*",
            "[删除](https://example.com)",
            "`删除`",
        ] {
            let p = Projection::new(source, false);
            assert_eq!(p.edit(source, 0..p.text.len(), ""), Some(String::new()));
        }
        assert_eq!(
            Projection::new("## 标题", true).changed("## 标题", "## 新标题"),
            Some("## 新标题".into())
        );
        assert_eq!(
            Projection::new("[链接](https://example.com)", false)
                .changed("[链接](https://example.com)", "新链接"),
            Some("[新链接](https://example.com)".into())
        );
    }
    #[test]
    fn image_only_paragraph_can_accept_first_input() {
        // A paragraph whose only content renders nothing (lone image / HR)
        // used to leave the run list empty, rejecting every keystroke.
        let p = Projection::new("![图](assets/a.png)", false);
        assert!(p.runs.len() == 1);
        assert_eq!(p.text, "");
        let next = p.changed("![图](assets/a.png)", "前言").unwrap();
        assert_eq!(next, "前言![图](assets/a.png)");
        let hr = Projection::new("---", false);
        assert_eq!(hr.changed("---", "上文").unwrap(), "上文---");
    }
    #[test]
    fn editing_preserves_horizontal_whitespace_and_cursor_positions() {
        for source in [
            "正文 ",
            "正文   ",
            "  正文  ",
            " ",
            "   ",
            "## 标题 ",
            "正文 **粗体**  ",
            "[链接](https://example.com) ",
        ] {
            let projection = Projection::new(source, true);
            assert!(
                projection
                    .text
                    .ends_with(if source.ends_with('\t') { "\t" } else { " " }),
                "missing whitespace: {source:?}"
            );
            assert_eq!(
                projection.source_cursor(projection.text.len()),
                Some(source.len())
            );
            let next = format!("{}字", projection.text);
            let updated = projection.changed(source, &next).unwrap();
            assert_eq!(updated, format!("{source}字"));
            assert_eq!(Projection::new(&updated, true).text, next);
        }
        assert_eq!(Projection::new("  正文  ", true).text, "  正文  ");
    }
    #[test]
    fn successive_spaces_and_undo_round_trip_without_losing_format() {
        let mut source = "正文".to_owned();
        for next in ["正文 ", "正文  ", "正文  字", "正文  ", "正文 ", "正文"] {
            source = Projection::new(&source, true)
                .changed(&source, next)
                .unwrap();
            assert_eq!(Projection::new(&source, true).text, next);
            assert_eq!(source, next);
        }
    }
    #[test]
    fn zero_content_can_accept_first_input() {
        assert_eq!(
            Projection::new("", false).edit("", 0..0, "你好"),
            Some("你好".into())
        );
    }
}
