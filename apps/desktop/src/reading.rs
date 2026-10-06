//! Native block rendering: parser byte ranges provide exact comment targets,
//! including repeated/format-rich paragraphs. No search in rendered text.
use crate::markdown::{DocMarkdown as Markdown, DocMarkdownWidgetRefExt};
use document_core::Workbench;
use makepad_widgets::*;
use pulldown_cmark::{Event as MdEvent, Options};
use std::ops::Range;

script_mod! {
    use mod.prelude.widgets.*
    let ArticleBlock = SolidView{width: Fill height: Fit flow: Down padding: 12 spacing: 6
                show_bg: true draw_bg.color: #xffffff
                markdown := mod.widgets.DocMarkdown{width: Fill height: Fit font_color: #x242a31 font_size: 14 heading_base_scale: 2.0 selectable: true
                    draw_selection +: {color: #xffd54f66}
                    splash_block := View{width: Fill height: Fit
                        splash_view := TextInput{width: Fill height: Fit is_multiline: true is_read_only: true empty_text: ""}
                    }
                }
                badge := ButtonFlatter{visible: false text: "查看批注" draw_text +: {color: #x95701c}}
            }
    mod.widgets.Reading = #(Reading::register_widget(vm)){
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down
            scroll_bar +: {bar_size: 12.0 bar_side_margin: 2.0
                draw_bg +: {size: 7.0 color: #x969ba5 color_hover: #x737b89 color_drag: #x566274 border_size: 0.0 border_radius: 3.5}
            }
            Block := ArticleBlock{}
            Highlight := ArticleBlock{
                draw_bg +: {color: #xffffff}
                badge +: {text: "💬 查看批注" draw_text +: {color: #x95701c}}
            }
            Empty := View{width: Fill height: 1}
        }
    }
}
#[derive(Clone, Debug)]
pub struct Block {
    pub range: Range<usize>,
    pub text: String,
    pub thread: Option<usize>,
}
#[derive(Clone, Debug, Default)]
pub enum ReadingAction {
    Open(usize),
    #[default]
    None,
}
#[derive(Script, ScriptHook, Widget)]
pub struct Reading {
    #[deref]
    view: View,
    #[rust]
    blocks: Vec<Block>,
    #[rust]
    text: String,
    /// Editor body size from Preferences; 0.0 falls back to the default.
    #[rust]
    body_size: f64,
    #[rust]
    pointer: Option<DVec2>,
    #[rust]
    highlights: Vec<Range<usize>>,
    #[rust]
    source_anchor: Option<(usize, usize)>,
    #[rust]
    cross_selection: Option<Range<usize>>,
}
/// Top-level Markdown blocks preserve complete list/table/fence syntax. Offset
/// ranges come from the same parser as the native Markdown widget.
pub fn block_ranges(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (event, range) in
        crate::markdown_parse::events(text, Options::ENABLE_TABLES | Options::ENABLE_MATH)
    {
        match event {
            MdEvent::Start(_) => {
                if depth == 0 {
                    start = range.start;
                }
                depth += 1;
            }
            MdEvent::End(_) => {
                depth -= 1;
                if depth == 0 {
                    result.push(start..range.end);
                }
            }
            _ if depth == 0 => result.push(range),
            _ => {}
        }
    }
    // Keep inter-block newlines outside the replaceable target: model output
    // usually has no final newline and must not join two neighboring blocks.
    for range in &mut result {
        while range.end > range.start && matches!(text.as_bytes()[range.end - 1], b'\n' | b'\r') {
            range.end -= 1;
        }
    }
    result.retain(|range| range.start < range.end);
    result
}
/// The document title is the first block when it is a single-line `# ` heading.
/// It lives in the title input, so editors and the reader do not render it.
pub fn title_block(text: &str) -> Option<Range<usize>> {
    let first = block_ranges(text).into_iter().next()?;
    let block = &text[first.clone()];
    (block.starts_with("# ") && !block.contains('\n')).then_some(first)
}
impl Reading {
    pub fn update(&mut self, cx: &mut Cx, workbench: &Workbench) {
        let changed = self.text != workbench.text();
        self.text = workbench.text().to_owned();
        let title = title_block(&self.text);
        self.highlights = (0..workbench.thread_count())
            .filter_map(|id| workbench.thread(id))
            .filter(|t| !t.resolved() && !t.processed() && t.revision() == workbench.revision())
            .map(|t| t.range())
            .collect();
        self.blocks = block_ranges(&self.text)
            .into_iter()
            .filter(|range| title.as_ref() != Some(range))
            .map(|range| {
                let thread = (0..workbench.thread_count()).find(|id| {
                    let t = workbench.thread(*id).unwrap();
                    !t.resolved()
                        && !t.processed()
                        && t.revision() == workbench.revision()
                        && t.range().start < range.end
                        && t.range().end > range.start
                });
                Block {
                    text: self.text[range.clone()].to_owned(),
                    range,
                    thread,
                }
            })
            .collect();
        if changed {
            self.source_anchor = None;
            self.cross_selection = None;
            self.view
                .portal_list(cx, ids!(list))
                .set_first_id_and_scroll(0, 0.0);
        }
        self.redraw(cx);
    }
    fn source_point(&self, cx: &Cx, point: DVec2) -> Option<(usize, usize)> {
        let list = self.view.portal_list(cx, ids!(list));
        let list = list.borrow()?;
        let (id, entry) = list
            .items()
            .iter()
            .filter(|(id, _)| **id < self.blocks.len())
            .min_by(|(_, a), (_, b)| {
                let distance = |r: Rect| {
                    if point.y < r.pos.y {
                        r.pos.y - point.y
                    } else if point.y > r.pos.y + r.size.y {
                        point.y - r.pos.y - r.size.y
                    } else {
                        0.0
                    }
                };
                distance(a.widget.area().rect(cx)).total_cmp(&distance(b.widget.area().rect(cx)))
            })?;
        let offset = entry
            .widget
            .child_by_path(ids!(markdown))
            .borrow::<Markdown>()?
            .source_at_point(cx, point)?;
        Some((*id, self.blocks[*id].range.start + offset))
    }
    pub fn selected_block(&self, cx: &Cx, position: DVec2) -> Option<Range<usize>> {
        if self
            .cross_selection
            .as_ref()
            .is_some_and(|r| r.start < r.end)
        {
            return self.cross_selection.clone();
        }
        let list = self.view.portal_list(cx, ids!(list));
        let list = list.borrow()?;
        for (id, entry) in list.items().iter() {
            if entry.widget.point_hits_area(cx, position) {
                let widget = entry.widget.child_by_path(ids!(markdown));
                if widget
                    .borrow::<Markdown>()
                    .is_some_and(|m| m.text_flow.has_selection())
                {
                    let range = widget.borrow::<Markdown>()?.precise_selection()?;
                    let block = self.blocks.get(*id)?;
                    return Some(block.range.start + range.start..block.range.start + range.end);
                }
            }
        }
        None
    }
    /// Live-apply a new body size (Preferences). Rendered blocks restyle on
    /// the next draw through `typography::apply`.
    pub fn set_body_size(&mut self, cx: &mut Cx, size: f64) {
        if (self.body_size - size).abs() >= 0.01 {
            self.body_size = size;
            self.view.redraw(cx);
        }
    }
    fn body(&self) -> f32 {
        if self.body_size > 0.0 {
            self.body_size as f32
        } else {
            crate::typography::BODY
        }
    }
    pub fn reveal(&mut self, cx: &mut Cx, range: Range<usize>) {
        if let Some(index) = self
            .blocks
            .iter()
            .position(|b| b.range.start < range.end && b.range.end > range.start)
        {
            self.view
                .portal_list(cx, ids!(list))
                .set_first_id_and_scroll(index, 0.0);
            self.redraw(cx);
        }
    }
}
impl Widget for Reading {
    fn text(&self) -> String {
        self.text.clone()
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.blocks.len());
                while let Some(id) = list.next_visible_item(cx) {
                    let block = self.blocks.get(id);
                    let row = list.item(
                        cx,
                        id,
                        if block.is_some_and(|b| b.thread.is_some()) {
                            id!(Highlight)
                        } else if block.is_some() {
                            id!(Block)
                        } else {
                            id!(Empty)
                        },
                    );
                    if let Some(block) = block {
                        let markdown = row.doc_markdown(cx, ids!(markdown));
                        if let Some(mut widget) = markdown.borrow_mut() {
                            crate::typography::apply(&mut widget, &block.text, self.body());
                            widget.external_selection =
                                self.cross_selection.as_ref().and_then(|s| {
                                    let a = s.start.max(block.range.start);
                                    let b = s.end.min(block.range.end);
                                    (a < b).then_some(
                                        a.saturating_sub(block.range.start)
                                            ..b.saturating_sub(block.range.start),
                                    )
                                });
                            widget.set_comment_ranges(
                                cx,
                                self.highlights
                                    .iter()
                                    .filter_map(|h| {
                                        let a = h.start.max(block.range.start);
                                        let b = h.end.min(block.range.end);
                                        (a < b).then_some(
                                            a.saturating_sub(block.range.start)
                                                ..b.saturating_sub(block.range.start),
                                        )
                                    })
                                    .collect(),
                            );
                        }
                        markdown.set_text(cx, &block.text);
                        row.button(cx, ids!(badge))
                            .set_visible(cx, block.thread.is_some());
                    }
                    row.draw_all_unscoped(cx);
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Event::Scroll(scroll) = event {
            let viewport = self.view.area().rect(cx);
            let selected = self
                .view
                .portal_list(cx, ids!(list))
                .borrow()
                .is_some_and(|list| {
                    list.items().values().any(|entry| {
                        entry
                            .widget
                            .child_by_path(ids!(markdown))
                            .borrow::<Markdown>()
                            .is_some_and(|m| m.text_flow.has_selection())
                    })
                });
            if selected
                && viewport.contains(scroll.abs)
                && crate::workspace::selection_edge_scroll(
                    viewport.pos.y,
                    viewport.size.y,
                    scroll.abs.y,
                ) == 0.0
            {
                scroll.handled_y.set(true);
                return;
            }
        }
        if let Event::TextCopy(copy) = event
            && let Some(range) = &self.cross_selection
        {
            *copy.response.borrow_mut() = self.text.get(range.clone()).map(str::to_owned);
            return;
        }
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            self.cross_selection = None;
            self.source_anchor = self.source_point(cx, mouse.abs);
            self.pointer = Some(mouse.abs);
        }
        let moved = match event {
            Event::MouseMove(m) => Some(m.abs),
            Event::MouseUp(m) if m.button == MouseButton::PRIMARY => Some(m.abs),
            _ => None,
        };
        if let Some(point) = moved
            && let Some((start_id, anchor)) = self.source_anchor
            && let Some((id, cursor)) = self.source_point(cx, point)
            && (id != start_id || self.cross_selection.is_some())
        {
            self.cross_selection = Some(anchor.min(cursor)..anchor.max(cursor));
            self.redraw(cx);
        }
        if let Event::MouseUp(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            self.source_anchor = None;
            let click = self
                .pointer
                .take()
                .is_some_and(|p| (p - mouse.abs).length() < 4.0);
            if click && let Some(list) = self.view.portal_list(cx, ids!(list)).borrow() {
                for (id, entry) in list.items().iter() {
                    if entry.widget.point_hits_area(cx, mouse.abs)
                        && let Some(thread) = self.blocks.get(*id).and_then(|b| b.thread)
                    {
                        cx.widget_action(self.widget_uid(), ReadingAction::Open(thread));
                        break;
                    }
                }
            }
        }
        self.view.handle_event(cx, event, scope);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_preserve_format_and_duplicate_blocks() {
        let text = "# 标题\n\n相同 **文字**\n\n相同 **文字**\n\n- a\n- b\n\n```rs\na()\n```\n";
        let ranges = block_ranges(text);
        assert_eq!(ranges.len(), 5);
        assert_eq!(text[ranges[1].clone()].trim(), "相同 **文字**");
        assert_eq!(text[ranges[2].clone()].trim(), "相同 **文字**");
        assert_ne!(ranges[1], ranges[2]);
        assert!(text[ranges[3].clone()].contains("- b"));
        assert!(text[ranges[4].clone()].contains("```"));
    }
}
