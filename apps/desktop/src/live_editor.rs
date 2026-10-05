//! Native presentation editing. Active text uses styled visible runs with a
//! lossless Markdown projection; source delimiters are never the input buffer.
use crate::edit_projection::Projection;
use crate::markdown::{DocMarkdown, DocMarkdownWidgetRefExt};
use crate::styled_input::{StyledInput, StyledInputRef, StyledInputWidgetRefExt};
use makepad_widgets::makepad_draw::text::selection::{Cursor, Selection};
use makepad_widgets::*;
use std::ops::Range;

script_mod! {
    use mod.prelude.widgets.*
    let ActiveLine = SolidView{width: Fill height: Fit flow: Down padding: Inset{left: 12 right: 12 top: 5 bottom: 5} draw_bg.color: #xffffff
        active_line := mod.widgets.StyledInput{width: Fill height: Fit is_multiline: true empty_text: "" blink_speed: 0.5
            draw_text +: {color: #x253041 color_focus: #x253041 color_hover: #x253041 text_style +: {font_size: 14}}
            draw_bg +: {color: #xffffff color_focus: #xffffff color_hover: #xffffff border_size: 0.0}
            draw_cursor +: {color: #x2563eb}
        }
    }
    let Line = SolidView{width: Fill height: Fit flow: Down padding: Inset{left: 12 right: 12 top: 5 bottom: 5}
        draw_bg.color: #xffffff
        rendered := mod.widgets.DocMarkdown{width: Fill height: Fit font_color: #x253041 font_size: 14 heading_base_scale: 2.0 selectable: false
            splash_block := View{width: Fill height: Fit
                splash_view := TextInput{width: Fill height: Fit is_multiline: true is_read_only: true empty_text: ""}
            }
        }
    }
    mod.widgets.LiveEditor = #(LiveEditor::register_widget(vm)){
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down keep_invisible: true selectable: false
            Render := Line{}
            RenderHighlight := Line{}
            Blank := SolidView{width: Fill height: 24 draw_bg.color: #xffffff}
            Active := ActiveLine{}
            ActiveHighlight := ActiveLine{}
            Empty := View{width: Fill height: 1}
        }
    }
}
#[derive(Clone, Debug, Default)]
pub enum LiveAction {
    Changed(String),
    Rejected,
    UndoRequested,
    SelectionReady(DVec2),
    #[default]
    None,
}
#[derive(Clone, Debug)]
struct DragSelection {
    down: MouseDownEvent,
    anchor: DVec2,
    cursor: DVec2,
    /// Word/line span the native input selected for a multi-click press
    /// (input-local indices). The drag anchors there instead of the press
    /// point, like a native word drag.
    anchor_span: Option<(usize, usize)>,
    applied_anchor: Option<Cursor>,
    applied_cursor: Option<(DVec2, Cursor)>,
    released: bool,
}
#[derive(Script, ScriptHook, Widget)]
pub struct LiveEditor {
    #[deref]
    view: View,
    #[rust]
    text: String,
    #[rust]
    ranges: Vec<Range<usize>>,
    #[rust]
    active: Option<usize>,
    #[rust]
    focus_pending: bool,
    #[rust]
    canvas_focus_pending: bool,
    #[rust]
    cursor: usize,
    #[rust]
    readonly: bool,
    #[rust]
    selected: Option<Range<usize>>,
    #[rust]
    drag: Option<DragSelection>,
    /// Unresolved, current-revision comment ranges: rows they overlap render
    /// with a yellow background in both rendered and active states.
    #[rust]
    highlights: Vec<Range<usize>>,
    /// Editor body size from Preferences; 0.0 means "not set yet" and falls
    /// back to `typography::BODY` so script-created defaults still work.
    #[rust]
    body_size: f64,
    /// Last primary press (position, time) for double/triple-click detection.
    /// `CxFingers::tap_count` is crate-private, so the editor keeps its own
    /// counter with the same thresholds (0.5s / 5px).
    #[rust]
    last_tap: Option<(DVec2, f64)>,
    #[rust]
    tap_count: u32,
    #[rust]
    cursor_frame: NextFrame,
    #[rust]
    drag_timer: Timer,
    #[rust]
    all_document: bool,
    #[rust]
    line_selected: bool,
}
pub fn units(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut offset = 0;
    let blocks = crate::reading::block_ranges(text);
    while offset < text.len() {
        // Preserve complete structural Markdown when rendering, not bogus
        // per-line table rows or unmatched code delimiters.
        let structural = blocks.iter().find(|b| b.start == offset).filter(|b| {
            let s = text[(*b).clone()].trim_start();
            s.starts_with("```")
                || s.starts_with("~~~")
                || s.starts_with('|')
                || s.starts_with("- ")
                || s.starts_with("* ")
                || s.starts_with("> ")
                || s.split_once(". ")
                    .is_some_and(|(prefix, _)| prefix.chars().all(|c| c.is_ascii_digit()))
        });
        let end = if let Some(range) = structural {
            range.end
        } else {
            text[offset..]
                .find('\n')
                .map(|n| offset + n)
                .unwrap_or(text.len())
        };
        result.push(offset..end);
        offset = if text.as_bytes().get(end) == Some(&b'\n') {
            end + 1
        } else {
            end
        };
    }
    if text.is_empty() || text.ends_with('\n') {
        result.push(text.len()..text.len());
    }
    result
}
/// Units the editor shows: the title block (and blank lines right after it) is
/// edited through the title input, never in the body.
pub fn visible_units(text: &str) -> Vec<Range<usize>> {
    let mut all = units(text);
    if let Some(title) = crate::reading::title_block(text)
        && all.first().is_some_and(|u| u.start == title.start)
    {
        if all.len() == 1 {
            let end = text.len();
            return std::iter::once(end..end).collect();
        }
        let blanks = all[1..].iter().take_while(|u| u.is_empty()).count();
        all.drain(..(1 + blanks).min(all.len() - 1));
    }
    all
}
impl LiveEditor {
    /// Buffer both gesture endpoints across render→source layout. Native
    /// TextInput hit-testing is invoked locally, never OS/App event re-entry.
    fn apply_drag(&mut self, cx: &mut Cx) -> bool {
        if self.focus_pending {
            return false;
        }
        let Some(drag) = self.drag.clone() else {
            return false;
        };
        // A released drag that can no longer resolve its input is finished:
        // drop it instead of letting the interval tick forever.
        let release_cleanup = |editor: &mut Self, cx: &mut Cx| {
            if drag.released {
                editor.drag = None;
                cx.stop_timer(editor.drag_timer);
            }
        };
        let Some(input) = self.input(cx) else {
            release_cleanup(self, cx);
            return false;
        };
        let Some(mut native) = input.borrow_mut() else {
            release_cleanup(self, cx);
            return false;
        };
        // Readiness = the row has been laid out and drawn. Do not probe the
        // caret cache (cursor_rect_in_absolute): it is only filled while the
        // input holds key focus, and an unfocused-but-dragging row would
        // otherwise deadlock — no caret cache, no selection, no focus regain.
        if !native.area().is_valid(cx) {
            drop(native);
            release_cleanup(self, cx);
            return false;
        }
        // Point→cursor mappings are cached for the whole gesture. Synthetic
        // presses capture the mouse onto this input and only the platform's
        // own mouse-up releases captures, so a synthetic press AFTER the real
        // release leaks a capture that would swallow the user's next click
        // via the captured-area fast path. Hence: mid-drag applies map freely
        // (the real release heals them), a released apply reuses the cache,
        // and only an instant gesture that completed before any move could be
        // applied maps after release — that leak is repaired on the next real
        // press (see the stale-capture guard in handle_event).
        let mapped = match (drag.applied_anchor, drag.applied_cursor) {
            (Some(anchor), Some((point, cursor))) if drag.released || point == drag.cursor => {
                Some((anchor, cursor))
            }
            (Some(anchor), Some((_, _))) => Some((
                anchor,
                nearest_cursor(&mut native, cx, &drag.down, drag.cursor),
            )),
            // Pure press (or sub-pixel wiggle) without an applied move: the
            // forwarded/native press already placed the caret; there is
            // nothing to select.
            _ if drag.released && (drag.anchor - drag.cursor).length() < 3.0 => None,
            _ => Some((
                match drag.anchor_span {
                    Some((start, _)) => Cursor {
                        index: start,
                        prefer_next_row: false,
                    },
                    None => nearest_cursor(&mut native, cx, &drag.down, drag.anchor),
                },
                nearest_cursor(&mut native, cx, &drag.down, drag.cursor),
            )),
        };
        let Some((anchor, cursor)) = mapped else {
            drop(native);
            cx.stop_timer(self.drag_timer);
            self.drag = None;
            cx.widget_action(self.widget_uid(), LiveAction::SelectionReady(drag.cursor));
            return true;
        };
        let selection = match drag.anchor_span {
            // Word/line drag: the anchor span stays fully selected; crossing
            // to its other side flips the anchor past it (native word-drag
            // behavior).
            Some((start, end)) if cursor.index <= start => Selection {
                anchor: Cursor {
                    index: end,
                    prefer_next_row: false,
                },
                cursor,
            },
            Some((start, end)) => Selection {
                anchor: Cursor {
                    index: start,
                    prefer_next_row: false,
                },
                cursor: Cursor {
                    index: cursor.index.max(end),
                    prefer_next_row: false,
                },
            },
            None => Selection { anchor, cursor },
        };
        native.set_selection_quiet(cx, selection);
        native.take_key_focus(cx);
        drop(native);
        if let Some(drag) = self.drag.as_mut() {
            drag.applied_anchor = Some(anchor);
            drag.applied_cursor = Some((drag.cursor, cursor));
        }
        if drag.released {
            cx.stop_timer(self.drag_timer);
            self.drag = None;
            cx.widget_action(self.widget_uid(), LiveAction::SelectionReady(drag.cursor));
        }
        true
    }
    pub fn cursor_offset(&self, cx: &Cx) -> Option<usize> {
        let range = self.ranges.get(self.active?)?;
        let input = self.input(cx)?;
        Projection::new(&self.text[range.clone()], true)
            .source_cursor(input.selection().cursor.index)
            .map(|i| range.start + i)
    }
    pub fn document_selected(&self) -> bool {
        self.all_document
    }
    fn clear_document_selection(&mut self, cx: &mut Cx) {
        self.all_document = false;
        self.selected = None;
        if let Some(input) = self.input(cx) {
            let cursor = input.selection().cursor;
            input.set_selection(
                cx,
                Selection {
                    anchor: cursor,
                    cursor,
                },
            );
        }
        self.redraw(cx);
    }
    fn replace_all(&mut self, cx: &mut Cx, text: &str) {
        self.update(cx, text);
        self.all_document = false;
        self.selected = None;
        self.activate_at(cx, text.len());
        cx.widget_action(self.widget_uid(), LiveAction::Changed(self.text.clone()));
    }
    pub fn set_highlights(&mut self, cx: &mut Cx, ranges: Vec<Range<usize>>) {
        if self.highlights != ranges {
            self.highlights = ranges;
            self.redraw(cx);
        }
    }
    pub fn update(&mut self, cx: &mut Cx, text: &str) {
        if self.text == text {
            return;
        }
        self.text = text.to_owned();
        self.canvas_focus_pending = false;
        self.all_document = false;
        self.line_selected = false;
        self.ranges = visible_units(text);
        self.active = None;
        self.focus_pending = false;
        cx.stop_timer(self.drag_timer);
        self.drag = None;
        self.selected = None;
        self.redraw(cx);
    }
    pub fn deactivate(&mut self, cx: &mut Cx) {
        self.canvas_focus_pending = false;
        self.selected = self.selection(cx);
        self.active = None;
        self.focus_pending = false;
        cx.stop_timer(self.drag_timer);
        self.drag = None;
        self.all_document = false;
        self.line_selected = false;
        self.ranges = visible_units(&self.text);
        self.redraw(cx);
    }
    pub fn set_readonly(&mut self, cx: &mut Cx, value: bool) {
        self.readonly = value;
        if value {
            cx.stop_timer(self.drag_timer);
            self.drag = None;
        }
        if let Some(input) = self.input(cx) {
            input.set_is_read_only(cx, value);
        }
        if let Some(list) = self.view.portal_list(cx, ids!(list)).borrow() {
            for (_, entry) in list.items().iter() {
                if let Some(mut markdown) = entry
                    .widget
                    .child_by_path(ids!(rendered))
                    .borrow_mut::<DocMarkdown>()
                {
                    markdown.set_cells_readonly(cx, value);
                }
            }
        }
        self.redraw(cx);
    }
    fn input(&self, cx: &Cx) -> Option<StyledInputRef> {
        let id = self.active?;
        let list = self.view.portal_list(cx, ids!(list));
        let list = list.borrow()?;
        let row = &list.items().iter().find(|(i, _)| **i == id)?.1.widget;
        Some(row.styled_input(cx, ids!(active_line)))
    }
    /// Effective body size in points (Preferences; default 14).
    pub fn body(&self) -> f64 {
        if self.body_size > 0.0 {
            self.body_size
        } else {
            crate::typography::BODY as f64
        }
    }
    /// Live-apply a new body size: the active line restyles on the next
    /// draw via `set_body_font_size`, rendered rows via `typography::apply`.
    pub fn set_body_size(&mut self, cx: &mut Cx, size: f64) {
        if (self.body_size - size).abs() < 0.01 {
            return;
        }
        self.body_size = size;
        if let Some(input) = self.input(cx)
            && let Some(mut native) = input.borrow_mut()
        {
            native.set_body_font_size(cx, self.body() as f32);
        }
        self.view.redraw(cx);
    }
    /// Force the active line to re-layout after an external font change
    /// (Preferences code-font override). Rendered rows re-layout through
    /// TextFlow on redraw; only the native input caches glyphs.
    pub fn restyle(&mut self, cx: &mut Cx) {
        if let Some(input) = self.input(cx)
            && let Some(mut native) = input.borrow_mut()
        {
            native.invalidate_layout(cx);
        }
        self.view.redraw(cx);
    }
    pub fn focused(&self, cx: &Cx) -> bool {
        if self.all_document {
            return true;
        }
        self.input(cx)
            .is_some_and(|input| input.borrow().is_some_and(|i| i.key_focus(cx)))
            || self
                .view
                .portal_list(cx, ids!(list))
                .borrow()
                .is_some_and(|list| {
                    list.items().iter().any(|(_, entry)| {
                        entry
                            .widget
                            .child_by_path(ids!(rendered))
                            .borrow::<DocMarkdown>()
                            .is_some_and(|m| m.cell_focused(cx))
                    })
                })
    }
    pub fn selection_anchor(&self, cx: &Cx) -> Option<DVec2> {
        let input = self.input(cx)?;
        input
            .cursor_rect_in_absolute(cx)
            .map(|rect| rect.pos + dvec2(0.0, rect.size.y))
    }
    pub fn selection(&self, cx: &Cx) -> Option<Range<usize>> {
        if self.all_document {
            return Some(0..self.text.len());
        }
        if self.active.is_none() {
            if let Some(list) = self.view.portal_list(cx, ids!(list)).borrow() {
                for (id, entry) in list.items().iter() {
                    if let Some(markdown) = entry
                        .widget
                        .child_by_path(ids!(rendered))
                        .borrow::<DocMarkdown>()
                        && let Some(range) = markdown.cell_selection(cx)
                        && let Some(block) = self.ranges.get(*id)
                    {
                        return Some(block.start + range.start..block.start + range.end);
                    }
                }
            }
            return self.selected.clone();
        }
        let input = self.input(cx)?;
        let selected = input.selection();
        if self.all_document
            && selected.start().index == 0
            && selected.end().index == input.text().len()
        {
            return Some(0..self.text.len());
        }
        let range = self.ranges.get(self.active?)?;
        if selected.start().index < selected.end().index {
            Projection::new(&self.text[range.clone()], !self.all_document)
                .source_range(selected.start().index..selected.end().index)
                .map(|r| range.start + r.start..range.start + r.end)
        } else {
            None
        }
    }
    pub fn activate_at(&mut self, cx: &mut Cx, offset: usize) {
        if self.readonly {
            return;
        }
        self.all_document = false;
        self.ranges = visible_units(&self.text);
        let id = self
            .ranges
            .iter()
            .position(|r| offset >= r.start && offset <= r.end)
            .unwrap_or(0);
        let start = self.ranges.get(id).map(|r| r.start).unwrap_or(0);
        self.line_selected = false;
        cx.stop_timer(self.drag_timer);
        self.drag = None;
        self.active = if self.text[self.ranges[id].clone()]
            .trim_start()
            .starts_with('|')
        {
            None
        } else {
            Some(id)
        };
        self.cursor = Projection::new(&self.text[self.ranges[id].clone()], true)
            .visible_cursor(offset.saturating_sub(start));
        self.focus_pending = true;
        self.view
            .portal_list(cx, ids!(list))
            .set_first_id_and_scroll(id, 0.0);
        self.redraw(cx);
    }
    fn activate(&mut self, cx: &mut Cx, id: usize, at_end: bool) {
        self.line_selected = false;
        self.selected = None;
        cx.stop_timer(self.drag_timer);
        self.drag = None;
        self.active = if self.text[self.ranges[id].clone()]
            .trim_start()
            .starts_with('|')
        {
            None
        } else {
            Some(id)
        };
        self.cursor = if at_end {
            Projection::new(&self.text[self.ranges[id].clone()], true)
                .text
                .len()
        } else {
            0
        };
        self.focus_pending = true;
        self.redraw(cx);
    }
}
impl Widget for LiveEditor {
    fn text(&self) -> String {
        self.text.clone()
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.ranges.len());
                while let Some(id) = list.next_visible_item(cx) {
                    let range = self.ranges.get(id);
                    let highlighted = range.is_some_and(|r| {
                        self.highlights
                            .iter()
                            .any(|h| h.start < r.end && h.end > r.start)
                    });
                    let template = if range.is_none() {
                        id!(Empty)
                    } else if self.active == Some(id) {
                        if highlighted {
                            id!(ActiveHighlight)
                        } else {
                            id!(Active)
                        }
                    } else if range.is_some_and(|r| r.is_empty()) {
                        id!(Blank)
                    } else if highlighted {
                        id!(RenderHighlight)
                    } else {
                        id!(Render)
                    };
                    let row = list.item(cx, id, template);
                    if let Some(range) = range {
                        let text = &self.text[range.clone()];
                        if self.active == Some(id) {
                            let input = row.styled_input(cx, ids!(active_line));
                            let projection = Projection::new(text, true);
                            if input.text() != projection.text {
                                input.set_text(cx, &projection.text);
                            }
                            if let Some(mut native) = input.borrow_mut() {
                                native.set_body_font_size(cx, self.body() as f32);
                                native.set_document_layout(cx, text);
                                native.set_runs(cx, projection.runs.clone());
                                native.comment_ranges = self
                                    .highlights
                                    .iter()
                                    .filter_map(|h| {
                                        let a = h.start.max(range.start);
                                        let b = h.end.min(range.end);
                                        (a < b)
                                            .then(|| {
                                                projection
                                                    .visible_range(a - range.start..b - range.start)
                                            })
                                            .flatten()
                                    })
                                    .collect();
                            }
                            input.set_is_read_only(cx, self.readonly);
                            if self.all_document
                                && let Some(mut native) = input.borrow_mut()
                            {
                                native.select_all(cx);
                            }
                            row.draw_all_unscoped(cx);
                            if self.focus_pending {
                                let cursor = if self.all_document {
                                    projection.text.len()
                                } else {
                                    self.cursor.min(projection.text.len())
                                };
                                input.set_selection_quiet(
                                    cx,
                                    Selection {
                                        cursor: Cursor {
                                            index: cursor,
                                            prefer_next_row: false,
                                        },
                                        anchor: Cursor {
                                            index: if self.all_document { 0 } else { cursor },
                                            prefer_next_row: false,
                                        },
                                    },
                                );
                                input.take_key_focus(cx);
                                if self.canvas_focus_pending {
                                    self.cursor_frame = cx.new_next_frame();
                                }
                                self.focus_pending = false;
                                if self.drag.is_some() {
                                    self.cursor_frame = cx.new_next_frame();
                                }
                            }
                        } else {
                            let markdown = row.doc_markdown(cx, ids!(rendered));
                            if let Some(mut widget) = markdown.borrow_mut() {
                                crate::typography::apply(&mut widget, text, self.body() as f32);
                                widget.comment_ranges = self
                                    .highlights
                                    .iter()
                                    .filter_map(|h| {
                                        let a = h.start.max(range.start);
                                        let b = h.end.min(range.end);
                                        (a < b).then_some(
                                            a.saturating_sub(range.start)
                                                ..b.saturating_sub(range.start),
                                        )
                                    })
                                    .collect();
                                widget.document_selected = self.all_document;
                                widget.editable_table = true;
                                widget.editor_readonly = self.readonly;
                            }
                            markdown.set_text(cx, text);
                            row.draw_all_unscoped(cx);
                        }
                    } else {
                        row.draw_all_unscoped(cx);
                    }
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if matches!(event,Event::KeyDown(key) if key.key_code==KeyCode::KeyZ && key.modifiers.is_primary() && !key.modifiers.shift && !key.is_repeat)
            && self.focused(cx)
            && !self.readonly
        {
            cx.widget_action(self.widget_uid(), LiveAction::UndoRequested);
            return;
        }
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            let repeat = self.last_tap.is_some_and(|(pos, time)| {
                mouse.time - time < 0.5 && (mouse.abs - pos).length() < 5.0
            });
            self.tap_count = if repeat { self.tap_count % 3 + 1 } else { 1 };
            self.last_tap = Some((mouse.abs, mouse.time));
        }
        if let Event::Scroll(scroll) = event {
            let selected = self.all_document
                || self.drag.is_some()
                || self.input(cx).is_some_and(|input| {
                    input.selection().start().index < input.selection().end().index
                });
            let viewport = self.view.area().rect(cx);
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
        if matches!(event, Event::MouseDown(_)) {
            // A newer click cancels the one-shot canvas focus hand-off,
            // so a fast subsequent click in title/comments is not stolen.
            self.canvas_focus_pending = false;
        }
        if matches!(event, Event::KeyDown(_)) {
            self.apply_drag(cx);
            self.drag = None;
            cx.stop_timer(self.drag_timer);
        }
        if self.all_document {
            match event {
                Event::TextCopy(copy) | Event::TextCut(copy) => {
                    *copy.response.borrow_mut() = Some(self.text.clone());
                    if matches!(event, Event::TextCut(_)) && !self.readonly {
                        self.replace_all(cx, "");
                    }
                    return;
                }
                Event::TextInput(input) if !self.readonly => {
                    self.replace_all(cx, &input.input);
                    return;
                }
                Event::KeyDown(key)
                    if matches!(key.key_code, KeyCode::Backspace | KeyCode::Delete)
                        && !self.readonly =>
                {
                    self.replace_all(cx, "");
                    return;
                }
                Event::MouseDown(_) | Event::KeyFocusLost(_) => {
                    self.clear_document_selection(cx);
                }
                Event::KeyDown(key)
                    if !(select_all_key(key) || key.modifiers.control || key.modifiers.logo) =>
                {
                    self.clear_document_selection(cx);
                }
                _ => {}
            }
        }
        if matches!(event, Event::MouseDown(_))
            || matches!(event, Event::KeyDown(key) if !select_all_key(key))
        {
            self.line_selected = false;
        }
        if let Event::KeyDown(key) = event
            && select_all_key(key)
            && let Some(input) = self.input(cx)
            && input.borrow().is_some_and(|i| i.key_focus(cx))
            && !self.readonly
        {
            if key.is_repeat {
                return;
            }
            if self.all_document {
                if let Some(mut native) = input.borrow_mut() {
                    native.select_all(cx);
                }
                return;
            }
            let line = source_line(&input.text(), input.selection().cursor.index);
            let selection = input.selection();
            if self.line_selected
                && selection.start().index == line.start
                && selection.end().index == line.end
            {
                // Shared source range, unchanged block tree. Each renderer
                // paints its own selection using its existing text geometry.
                self.all_document = true;
                if let Some(mut native) = input.borrow_mut() {
                    native.select_all(cx);
                }
                cx.stop_timer(self.drag_timer);
                self.drag = None;
                self.redraw(cx);
            } else {
                input.set_selection_quiet(
                    cx,
                    Selection {
                        anchor: Cursor {
                            index: line.start,
                            prefer_next_row: false,
                        },
                        cursor: Cursor {
                            index: line.end,
                            prefer_next_row: false,
                        },
                    },
                );
                self.line_selected = true;
            }
            return;
        }
        if self.cursor_frame.is_event(event).is_some() || self.drag_timer.is_event(event).is_some()
        {
            if self.drag_timer.is_event(event).is_some()
                && let Some(drag) = self.drag.as_ref()
                && !drag.released
            {
                let viewport = self.view.area().rect(cx);
                let delta = crate::workspace::selection_edge_scroll(
                    viewport.pos.y,
                    viewport.size.y,
                    drag.cursor.y,
                );
                if delta != 0.0 {
                    let list = self.view.portal_list(cx, ids!(list));
                    if let Some(mut native) = list.borrow_mut() {
                        let id = native.first_id();
                        let offset = native.first_scroll();
                        native.set_first_id_and_scroll(id, offset + delta);
                        native.redraw(cx);
                    }
                }
            }
            if self.canvas_focus_pending
                && let Some(input) = self.input(cx)
            {
                input.take_key_focus(cx);
                self.canvas_focus_pending = false;
            }
            self.apply_drag(cx);
        }
        if self.drag.is_some() {
            match event {
                Event::MouseMove(mouse) => {
                    self.drag.as_mut().unwrap().cursor = mouse.abs;
                    self.apply_drag(cx);
                    return;
                }
                Event::MouseUp(mouse) if mouse.button == MouseButton::PRIMARY => {
                    let drag = self.drag.as_mut().unwrap();
                    drag.cursor = mouse.abs;
                    drag.released = true;
                    self.apply_drag(cx);
                    return;
                }
                Event::MouseDown(_) | Event::Scroll(_) => {
                    cx.stop_timer(self.drag_timer);
                    self.drag = None;
                }
                _ => {}
            }
        }
        if !self.readonly
            && let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            let link_hit = self
                .view
                .portal_list(cx, ids!(list))
                .borrow()
                .is_some_and(|list| {
                    list.items().iter().any(|(_, entry)| {
                        entry
                            .widget
                            .child_by_path(ids!(rendered))
                            .borrow::<DocMarkdown>()
                            .is_some_and(|m| m.link_at(cx, mouse.abs))
                    })
                });
            if link_hit && (mouse.modifiers.logo || mouse.modifiers.control) {
                self.view.handle_event(cx, event, scope);
                return;
            }
            let hit = self
                .view
                .portal_list(cx, ids!(list))
                .borrow()
                .and_then(|list| {
                    list.items()
                        .iter()
                        .find(|(_, e)| e.widget.point_hits_area(cx, mouse.abs))
                        .map(|(id, _)| *id)
                });
            if hit.is_none_or(|id| id >= self.ranges.len())
                && self.ranges.iter().all(Range::is_empty)
                && !self.ranges.is_empty()
                && self.view.area().rect(cx).contains(mouse.abs)
            {
                // The empty document is a writing canvas, not a 24px target.
                // Title updates deactivate/recreate its row; filler items
                // below it must never make the rest of the body unclickable.
                self.activate(cx, self.ranges.len() - 1, false);
                self.selected = None;
                self.canvas_focus_pending = true;
                self.last_tap = None;
                return;
            }
            if hit.is_none()
                && let Some(input) = self.input(cx)
                && cx.fingers.is_area_captured(input.area())
            {
                // Stale capture: a post-release drag apply pressed this input
                // synthetically, and captures are only released by the
                // platform's own mouse-up. Left alone, hit-testing's
                // captured-area fast path would feed THIS press to the input
                // wherever it landed — stealing key focus and swallowing
                // clicks aimed at other widgets. Keep the press out of the
                // editor subtree; the real target handles it and moves key
                // focus, which deactivates the row via key_focus_lost. The
                // platform's mouse-up clears the stale capture afterwards.
                return;
            }
            if let Some(id) = hit
                && id < self.ranges.len()
            {
                {
                    if self.text[self.ranges[id].clone()]
                        .trim_start()
                        .starts_with('|')
                    {
                        self.active = None;
                        self.view.handle_event(cx, event, scope);
                        return;
                    }
                    if self.active != Some(id) {
                        self.activate(cx, id, false);
                    } else {
                        // Press on the already-active row: hand it to the native
                        // input first so it places the caret (or word-/line-selects
                        // on multi-click), takes key focus and holds the real
                        // pointer capture (marking the event handled keeps
                        // ancestor views from grabbing key focus). Drag moves are
                        // still applied synthetically — native TextInput drag
                        // hit-testing can drop pointer moves once the gesture
                        // crosses wrapped visual lines, freezing the selection
                        // mid-paragraph — and the synthetic presses then reuse
                        // that real capture instead of leaking their own.
                        self.line_selected = false;
                        self.selected = None;
                        self.view.handle_event(cx, event, scope);
                    }
                    // After a forwarded multi-click press the native input holds
                    // a word/line selection; anchor the drag on its span so the
                    // gesture extends that selection instead of restarting it.
                    let anchor_span = if self.active == Some(id) && self.tap_count >= 2 {
                        self.input(cx).and_then(|input| {
                            let selection = input.selection();
                            (selection.start().index < selection.end().index)
                                .then(|| (selection.start().index, selection.end().index))
                        })
                    } else {
                        None
                    };
                    cx.stop_timer(self.drag_timer);
                    self.drag_timer = cx.start_interval(0.016);
                    self.drag = Some(DragSelection {
                        down: mouse.clone(),
                        anchor: mouse.abs,
                        cursor: mouse.abs,
                        anchor_span,
                        applied_anchor: None,
                        applied_cursor: None,
                        released: false,
                    });
                    return;
                }
            }
        }
        if let Event::Actions(actions) = event {
            let rows = self
                .view
                .portal_list(cx, ids!(list))
                .items_with_actions(actions);
            for (id, row) in rows {
                if let Some(action) =
                    actions.find_widget_action(row.child_by_path(ids!(rendered)).widget_uid())
                    && let crate::markdown::TableAction::Rejected = action.cast()
                {
                    cx.widget_action(self.widget_uid(), LiveAction::Rejected);
                }
                if let Some(action) =
                    actions.find_widget_action(row.child_by_path(ids!(rendered)).widget_uid())
                    && let crate::markdown::TableAction::Changed(block) = action.cast()
                    && !self.readonly
                {
                    self.text.replace_range(self.ranges[id].clone(), &block);
                    self.ranges = visible_units(&self.text);
                    cx.widget_action(self.widget_uid(), LiveAction::Changed(self.text.clone()));
                    self.redraw(cx);
                }
            }
        }
        // First transfer native input changes into our full-document buffer,
        // then emit a domain command. Never set_text on an unchanged active IME.
        if let Some(input) = self.input(cx)
            && let Event::Actions(actions) = event
        {
            if let Some(line) = input.changed(actions)
                && let Some(id) = self.active
            {
                let range = self.ranges[id].clone();
                let projection = Projection::new(&self.text[range.clone()], !self.all_document);
                let cursor = input.selection().cursor.index;
                let replacement = projection.changed(&self.text[range.clone()], &line);
                let Some(replacement) = replacement else {
                    input.set_text(cx, &projection.text);
                    cx.widget_action(self.widget_uid(), LiveAction::Rejected);
                    return;
                };
                let updated = Projection::new(&replacement, !self.all_document);
                let global =
                    range.start + updated.source_cursor(cursor).unwrap_or(replacement.len());
                self.text.replace_range(range.clone(), &replacement);
                self.all_document = false;
                self.line_selected = false;
                self.ranges = visible_units(&self.text);
                let next = self
                    .ranges
                    .iter()
                    .position(|r| global >= r.start && global <= r.end)
                    .unwrap_or(self.ranges.len() - 1);
                self.active = Some(next);
                self.cursor = Projection::new(&self.text[self.ranges[next].clone()], true)
                    .visible_cursor(global - self.ranges[next].start);
                if next != id || self.ranges[next].len() != replacement.len() {
                    self.focus_pending = true;
                }
                self.selected = None;
                cx.widget_action(self.widget_uid(), LiveAction::Changed(self.text.clone()));
                self.redraw(cx);
            }
            if input.key_focus_lost(actions)
                && !self.canvas_focus_pending
                && !self.focus_pending
                && self.drag.is_none()
                && self.input(cx).is_some_and(|current| {
                    current.widget_uid() == input.widget_uid()
                        && !current.borrow().is_some_and(|native| native.key_focus(cx))
                })
            {
                self.selected = self.selection(cx);
                self.active = None;
                self.line_selected = false;
                self.all_document = false;
                self.ranges = visible_units(&self.text);
                self.redraw(cx);
            }
        }
        if let Some(input) = self.input(cx)
            && let Event::KeyDown(key) = event
            && input.borrow().is_some_and(|i| i.key_focus(cx))
            && !self.readonly
        {
            let id = self.active.unwrap();
            let selection = input.selection();
            let start = selection.cursor.index == 0;
            let end = selection.cursor.index == input.text().len();
            if key.key_code == KeyCode::ArrowUp
                && id > 0
                && (start || !input.text().contains('\n'))
                && !key.modifiers.shift
            {
                self.activate(cx, id - 1, true);
                return;
            }
            if key.key_code == KeyCode::ArrowDown
                && id + 1 < self.ranges.len()
                && (end || !input.text().contains('\n'))
                && !key.modifiers.shift
            {
                self.activate(cx, id + 1, false);
                return;
            }
            if key.key_code == KeyCode::Backspace
                && start
                && selection.start().index == selection.end().index
                && id > 0
            {
                let offset = self.ranges[id].start;
                if offset > 0 && self.text.as_bytes()[offset - 1] == b'\n' {
                    self.text.remove(offset - 1);
                    self.ranges = visible_units(&self.text);
                    self.activate_at(cx, offset - 1);
                    cx.widget_action(self.widget_uid(), LiveAction::Changed(self.text.clone()));
                    return;
                }
            }
        }
        self.view.handle_event(cx, event, scope);
    }
}
fn nearest_cursor(
    input: &mut StyledInput,
    cx: &mut Cx,
    down: &MouseDownEvent,
    point: DVec2,
) -> Cursor {
    // Use native hit-testing's current laid-out glyph map. cursor_rect is a
    // draw-frame cache and does NOT update when move_cursor_right is called.
    let mut event = down.clone();
    event.handled.set(Area::Empty);
    let rect = input.area().rect(cx);
    event.abs = dvec2(
        point
            .x
            .clamp(rect.pos.x + 1.0, rect.pos.x + rect.size.x - 1.0),
        point
            .y
            .clamp(rect.pos.y + 1.0, rect.pos.y + rect.size.y - 1.0),
    );
    input.handle_event(cx, &Event::MouseDown(event.clone()), &mut Scope::empty());
    // Use MouseMove to avoid click-count (double/triple click) word expansion.
    input.handle_event(
        cx,
        &Event::MouseMove(MouseMoveEvent {
            abs: event.abs,
            lock_delta: dvec2(0.0, 0.0),
            window_id: event.window_id,
            modifiers: KeyModifiers::default(),
            time: event.time + 0.001,
            handled: std::cell::Cell::new(Area::Empty),
        }),
        &mut Scope::empty(),
    );
    let cursor = input.cursor();
    input.handle_event(
        cx,
        &Event::MouseUp(MouseUpEvent {
            abs: event.abs,
            button: event.button,
            window_id: event.window_id,
            modifiers: KeyModifiers::default(),
            time: event.time + 0.002,
        }),
        &mut Scope::empty(),
    );
    cursor
}
pub fn select_all_key(key: &KeyEvent) -> bool {
    key.key_code == KeyCode::KeyA
        && (key.modifiers.control || key.modifiers.logo)
        && !key.modifiers.alt
        && !key.modifiers.shift
}

/// Cmd+, (macOS) or Ctrl+, (other platforms) opens Preferences.
pub fn preferences_key(key: &KeyEvent) -> bool {
    key.key_code == KeyCode::Comma
        && (key.modifiers.control || key.modifiers.logo)
        && !key.modifiers.alt
        && !key.modifiers.shift
}

/// Cmd+O (macOS) or Ctrl+O opens the quick switcher.
pub fn switcher_key(key: &KeyEvent) -> bool {
    key.key_code == KeyCode::KeyO
        && (key.modifiers.control || key.modifiers.logo)
        && !key.modifiers.alt
        && !key.modifiers.shift
}

/// Feishu-style comment shortcut: Cmd+Shift+M (macOS) or Ctrl+Shift+M.
/// Adds a comment on the current text selection.
pub fn comment_shortcut_key(key: &KeyEvent) -> bool {
    key.key_code == KeyCode::KeyM
        && key.modifiers.shift
        && (key.modifiers.control || key.modifiers.logo)
        && !key.modifiers.alt
}
pub fn source_line(text: &str, cursor: usize) -> Range<usize> {
    let cursor = cursor.min(text.len());
    let start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[cursor..]
        .find('\n')
        .map(|i| cursor + i)
        .unwrap_or(text.len());
    start..end
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_units_keep_empty_lines_and_structures() {
        let text = "# 标题\n\n正文 **粗体**\n下一行\n\n```rs\na()\n```\n";
        let ranges = units(text);
        assert_eq!(ranges.len(), 7);
        assert_eq!(&text[ranges[2].clone()], "正文 **粗体**");
        assert!(text[ranges[5].clone()].contains("a()"));
        assert!(ranges.last().unwrap().is_empty());
    }
    #[test]
    fn title_is_not_a_visible_unit() {
        let text = "# 标题\n\n正文\n";
        let ranges = visible_units(text);
        assert_eq!(&text[ranges[0].clone()], "正文");
        let only_title = "# 标题\n\n";
        assert_eq!(visible_units(only_title), vec![10..10]);
        let plain = "正文\n";
        assert_eq!(visible_units(plain), units(plain));
    }
}
