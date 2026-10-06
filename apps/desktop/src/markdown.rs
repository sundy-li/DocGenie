//! App-owned native renderer based on the pinned Makepad Markdown widget.
//! Upstream copyright / license attribution: see THIRD_PARTY_NOTICES.md.
//! Local image references never cause network requests.
use makepad_widgets::*;
use project_store::library::Library;
use std::collections::HashMap;

use pulldown_cmark::{
    Alignment, CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    mod.widgets.DocMarkdownLinkBase = #(DocMarkdownLink::register_widget(vm))

    mod.widgets.DocMarkdownBase = #(DocMarkdown::register_widget(vm))

    mod.widgets.DocMarkdownLink = set_type_default() do mod.widgets.DocMarkdownLinkBase{
        width: Fit height: Fit
        align: Align{x: 0. y: 0.}

        label_walk: Walk{width: Fit height: Fit}

        draw_icon +: {
            hover: instance(0.0)
            pressed: instance(0.0)

            get_color: fn() {
                return mix(
                    mix(
                        theme.color_label_inner,
                        theme.color_label_inner_hover,
                        self.hover
                    ),
                    theme.color_label_inner_down,
                    self.pressed
                )
            }
        }

        animator: Animator{
            hover: {
                default: @off
                off: AnimatorState{
                    from: {all: Forward {duration: 0.1}}
                    apply: {
                        draw_bg: {pressed: 0.0 hover: 0.0}
                        draw_icon: {pressed: 0.0 hover: 0.0}
                        draw_text: {pressed: 0.0 hover: 0.0}
                    }
                }

                on: AnimatorState{
                    from: {
                        all: Forward {duration: 0.1}
                        pressed: Forward {duration: 0.01}
                    }
                    apply: {
                        draw_bg: {pressed: 0.0 hover: snap(1.0)}
                        draw_icon: {pressed: 0.0 hover: snap(1.0)}
                        draw_text: {pressed: 0.0 hover: snap(1.0)}
                    }
                }

                pressed: AnimatorState{
                    from: {all: Forward {duration: 0.2}}
                    apply: {
                        draw_bg: {pressed: snap(1.0) hover: 1.0}
                        draw_icon: {pressed: snap(1.0) hover: 1.0}
                        draw_text: {pressed: snap(1.0) hover: 1.0}
                    }
                }
            }
        }

        draw_bg +: {
            pressed: instance(0.0)
            hover: instance(0.0)

            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let offset_y = 1.0
                sdf.move_to(0. self.rect_size.y-offset_y)
                sdf.line_to(self.rect_size.x self.rect_size.y-offset_y)
                return sdf.stroke(mix(
                    theme.color_label_inner,
                    theme.color_label_inner_down,
                    self.pressed
                ), mix(0.0, 0.8, self.hover))
            }
        }

        draw_text +: {
            pressed: instance(0.0)
            hover: instance(0.0)

            color_hover: uniform(theme.color_label_inner_hover)
            color_pressed: uniform(theme.color_label_inner_down)

            color: theme.color_label_inner
            text_style: theme.font_regular{
                font_size: theme.font_size_p
            }
            get_color: fn() {
                return mix(
                    mix(
                        self.color,
                        self.color_hover,
                        self.hover
                    ),
                    self.color_pressed,
                    self.pressed
                )
            }
        }
    }

    mod.widgets.DocMarkdown = set_type_default() do mod.widgets.DocMarkdownBase{
        width: Fill height: Fit
        flow: Flow.Right{wrap: true}
        padding: theme.mspace_1

        font_size: theme.font_size_p
        font_color: theme.color_label_inner

        paragraph_spacing: 16
        pre_code_spacing: 8
        inline_code_padding: theme.mspace_1
        inline_code_margin: theme.mspace_1
        heading_base_scale: 1.8

        draw_text +: {
            color: theme.color_label_inner
        }

        text_style_normal: theme.font_regular{
            font_size: theme.font_size_p
        }

        text_style_italic: theme.font_italic{
            font_size: theme.font_size_p
        }

        text_style_bold: theme.font_bold{
            font_size: theme.font_size_p
        }

        text_style_bold_italic: theme.font_bold_italic{
            font_size: theme.font_size_p
        }

        text_style_fixed: theme.font_code{
            font_size: theme.font_size_p
        }

        code_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{left: theme.space_3, right: theme.space_3, top: theme.space_2, bottom: 10}
        }
        code_walk: Walk{width: Fill height: Fit}

        quote_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{left: theme.space_3, right: theme.space_3, top: theme.space_2, bottom: theme.space_2}
        }
        quote_walk: Walk{width: Fill height: Fit}

        list_item_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: theme.mspace_1
        }
        list_item_walk: Walk{
            height: Fit width: Fill
        }

        sep_walk: Walk{
            width: Fill height: 4.
            margin: theme.mspace_v_1
        }

        draw_block +: {
            line_color: theme.color_label_inner
            sep_color: theme.color_shadow
            // A Markdown quote is a neutral structural surface, not a
            // comment annotation. Yellow is reserved for exact text bands.
            quote_bg_color: #xf5f6f8
            quote_fg_color: theme.color_label_inner
            code_color: theme.color_bg_highlight
            selection_color: theme.color_selection_focus
            table_header_bg_color: theme.color_bg_highlight
            table_border_color: theme.color_shadow
            space_1: uniform(theme.space_1)
            space_2: uniform(theme.space_2)
        }

        link := mod.widgets.DocMarkdownLink{
            draw_text +: {color: #x2563eb color_hover: #x1d4ed8 color_pressed: #x1e40af}
        }
        image := Image{width: Fill height: 240 fit: ImageFit.Smallest}
        cell := mod.widgets.StyledInput{width: Fill height: Fit is_multiline: false empty_text: " "
            padding: 4
            draw_bg +: {color: #xffffff color_hover: #xf8fafc color_focus: #xffffff border_size: 0.0}
            draw_text +: {color: #x253041 color_hover: #x253041 color_focus: #x253041 text_style +: {font_size: 14}}
            draw_cursor +: {color: #x2563eb}
        }
    }
}

/// The state of a list at a given nesting level.
struct ListState {
    // Current item number for ordered lists.
    current_number: u64,
    // Start number for ordered lists, None for unordered.
    start_number: Option<u64>,
}

#[derive(Script, ScriptHook, Widget)]
pub struct DocMarkdown {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    pub text_flow: TextFlow,
    #[live]
    body: ArcStringMut,
    #[live]
    paragraph_spacing: f64,
    #[live]
    pre_code_spacing: f64,
    #[live(false)]
    use_code_block_widget: bool,
    #[rust]
    in_code_block: bool,
    #[rust]
    code_block_string: String,
    #[rust]
    in_splash_block: bool,
    #[rust]
    splash_block_string: String,
    #[live(false)]
    use_math_widget: bool,
    #[rust]
    auto_id: u64,
    #[live]
    heading_base_scale: f64,
    #[rust]
    images: HashMap<u64, String>,
    #[rust]
    links: Vec<WidgetRef>,
    #[live(false)]
    pub editable_table: bool,
    #[rust]
    pub editor_readonly: bool,
    #[rust]
    pub document_selected: bool,
    #[live]
    document_band: DrawColor,
    #[live]
    comment_band: DrawColor,
    #[rust]
    pub comment_ranges: Vec<std::ops::Range<usize>>,
    #[rust]
    pub editor_selection: bool,
    #[rust]
    pub external_selection: Option<std::ops::Range<usize>>,
    #[rust]
    comment_areas: Vec<Area>,
    #[rust]
    selection_areas: Vec<Area>,
    #[live]
    range_band: DrawColor,
    #[rust]
    source_runs: Vec<crate::edit_projection::Run>,
    #[rust]
    selection_anchor: Option<usize>,
    #[rust]
    selected_source: Option<std::ops::Range<usize>>,
    /// Native selection segments capture turtle coordinates before parent
    /// alignment/virtual-list shifts. Translate pointer into that draw frame.
    #[rust]
    selection_origin: DVec2,
    #[rust]
    had_document_selection: bool,
    #[rust]
    cells: Vec<(
        WidgetRef,
        std::ops::Range<usize>,
        crate::edit_projection::Projection,
    )>,
}

impl Widget for DocMarkdown {
    fn is_interactive(&self) -> bool {
        false
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.text_flow.selectable {
            let point = match event {
                Event::MouseDown(m) if m.button == MouseButton::PRIMARY => Some((m.abs, true)),
                Event::MouseMove(m) => Some((m.abs, false)),
                Event::MouseUp(m) if m.button == MouseButton::PRIMARY => Some((m.abs, false)),
                _ => None,
            };
            if let Some((point, down)) = point {
                if down {
                    self.selection_anchor = self.text_flow.selection_point_to_char_index(cx, point);
                    self.selected_source = None;
                }
                if let (Some(anchor), Some(cursor)) = (
                    self.selection_anchor,
                    self.text_flow.selection_point_to_char_index(cx, point),
                ) {
                    let projection = crate::edit_projection::Projection {
                        text: self.text_flow.selection_get_full_text(),
                        runs: self.source_runs.clone(),
                    };
                    self.selected_source = projection
                        .comment_range(self.body.as_ref(), anchor.min(cursor)..anchor.max(cursor));
                }
                if matches!(event, Event::MouseUp(_)) {
                    self.selection_anchor = None;
                }
            }
        }
        self.text_flow.handle_event(cx, event, scope);
        if self.text_flow.selectable
            && matches!(event,Event::KeyDown(key) if key.key_code==KeyCode::KeyA && key.modifiers.is_primary())
        {
            let projection = crate::edit_projection::Projection {
                text: self.text_flow.selection_get_full_text(),
                runs: self.source_runs.clone(),
            };
            self.selected_source =
                projection.comment_range(self.body.as_ref(), 0..projection.text.len());
        }
        if let Event::Actions(actions) = event {
            use crate::styled_input::StyledInputWidgetRefExt;
            for (widget, range, projection) in &self.cells {
                let input = widget.as_styled_input();
                if let Some(text) = input.changed(actions) {
                    let source = &self.body.as_ref()[range.clone()];
                    // A literal pipe cannot become a new column accidentally.
                    if !text.contains(['|', '\n', '\r'])
                        && let Some(replacement) = projection.changed(source, &text)
                    {
                        let mut updated = self.body.as_ref().to_owned();
                        updated.replace_range(range.clone(), &replacement);
                        cx.widget_action(self.widget_uid(), TableAction::Changed(updated));
                    } else {
                        input.set_text(cx, &projection.text);
                        cx.widget_action(self.widget_uid(), TableAction::Rejected);
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.auto_id = 0;
        self.links.clear();
        self.cells.clear();
        self.comment_areas.clear();
        self.selection_areas.clear();
        self.source_runs.clear();

        if self.editor_selection {
            self.text_flow.selectable = true;
        }
        self.begin(cx, walk);
        if self.document_selected {
            self.text_flow.areas_tracker.push_tracker();
        }
        if self.editable_table && self.body.as_ref().trim_start().starts_with('|') {
            self.draw_editable_table(cx);
        } else {
            self.process_markdown_doc(cx);
        }
        if self.editor_selection {
            // Source-run bands below reuse actual draw geometry. Do not also
            // paint TextFlow's independent tracker selection over them.
            self.text_flow.clear_selection();
        }
        self.end(cx);
        if self.document_selected {
            // Capture actual drawn row rectangles, including list markers.
            // TextFlow's independent selection layout diverges inside list
            // turtles; do not recompute glyph geometry for whole-block bands.
            let (start, end) = self.text_flow.areas_tracker.pop_tracker();
            let rects: Vec<_> = self.text_flow.areas_tracker.areas[start..end]
                .iter()
                .map(|area| area.rect(cx))
                .collect();
            self.document_band.color = vec4(105.0 / 255.0, 175.0 / 255.0, 165.0 / 255.0, 0.2);
            for rect in rects {
                self.document_band.draw_abs(cx, rect);
            }
            for link in &self.links {
                self.document_band.draw_abs(cx, link.area().rect(cx));
            }
        }
        self.range_band.color = vec4(105.0 / 255.0, 175.0 / 255.0, 165.0 / 255.0, 0.2);
        for area in &self.selection_areas {
            self.range_band.draw_abs(cx, area.rect(cx));
        }
        self.comment_band.color = vec4(1.0, 0.77, 0.03, 0.25);
        for area in &self.comment_areas {
            self.comment_band.draw_abs(cx, area.rect(cx));
        }
        self.selection_origin = self.text_flow.area().rect(cx).pos;
        self.had_document_selection = self.document_selected;

        DrawStep::done()
    }

    fn text(&self) -> String {
        self.body.as_ref().to_string()
    }

    fn set_text(&mut self, cx: &mut Cx, v: &str) {
        if self.body.as_ref() != v {
            self.images.clear();
            self.body.set(v);
            self.redraw(cx);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum TableAction {
    Changed(String),
    Rejected,
    #[default]
    None,
}
#[allow(clippy::too_many_arguments)]
fn draw_source_text(
    tf: &mut TextFlow,
    cx: &mut Cx2d,
    text: &str,
    source: std::ops::Range<usize>,
    comments: &[std::ops::Range<usize>],
    areas: &mut Vec<Area>,
    runs: &mut Vec<crate::edit_projection::Run>,
    selection: Option<&std::ops::Range<usize>>,
    selected_areas: &mut Vec<Area>,
) {
    let start = tf.selection_text_len();
    runs.push(crate::edit_projection::Run {
        visible: start..start + text.len(),
        source: source.clone(),
        style: Default::default(),
    });
    let mut boundaries = vec![0, text.len()];
    if source.len() == text.len() {
        for comment in comments.iter().chain(selection) {
            let a = comment.start.max(source.start);
            let b = comment.end.min(source.end);
            if a < b {
                boundaries.extend([a - source.start, b - source.start]);
            }
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    for pair in boundaries.windows(2) {
        let Some(part) = text.get(pair[0]..pair[1]) else {
            continue;
        };
        let marked = comments
            .iter()
            .any(|c| c.start < source.start + pair[1] && c.end > source.start + pair[0]);
        let selected = selection
            .is_some_and(|s| s.start < source.start + pair[1] && s.end > source.start + pair[0]);
        if marked || selected {
            tf.areas_tracker.push_tracker();
        }
        tf.draw_text(cx, part);
        if marked || selected {
            let (a, b) = tf.areas_tracker.pop_tracker();
            if marked {
                areas.extend_from_slice(&tf.areas_tracker.areas[a..b]);
            }
            if selected {
                selected_areas.extend_from_slice(&tf.areas_tracker.areas[a..b]);
            }
        }
    }
}
impl DocMarkdown {
    pub fn set_comment_ranges(&mut self, cx: &mut Cx, ranges: Vec<std::ops::Range<usize>>) {
        if self.comment_ranges != ranges {
            self.comment_ranges = ranges;
            self.redraw(cx);
        }
    }
    pub fn source_at_point(&self, cx: &Cx, point: DVec2) -> Option<usize> {
        // LinkLabel's native WidgetText hit-map interpolates by y, not glyph
        // x. Do not invent a partial-link endpoint from that approximation.
        if self
            .links
            .iter()
            .any(|link| link.point_hits_area(cx, point))
        {
            return None;
        }
        if self.text_flow.selection_get_full_text().len() != self.text_flow.selection_text_len() {
            return None;
        }
        let point = point - (self.text_flow.area().rect(cx).pos - self.selection_origin);
        let index = self.text_flow.selection_point_to_char_index(cx, point)?;
        let run = self
            .source_runs
            .iter()
            .find(|r| r.visible.start <= index && r.visible.end >= index)?;
        if run.visible.len() != run.source.len() {
            return None;
        }
        Some(run.source.start + index - run.visible.start)
    }
    pub fn precise_selection(&self) -> Option<std::ops::Range<usize>> {
        // TextFlow's public text omits object-replacement gaps, while indices
        // include them. Refuse unmapped complex child content rather than
        // silently shifting a comment onto the wrong bytes.
        if self.text_flow.selection_get_full_text().len() != self.text_flow.selection_text_len() {
            return None;
        }
        self.selected_source.clone()
    }
    pub fn set_cells_readonly(&mut self, cx: &mut Cx, value: bool) {
        use crate::styled_input::StyledInputWidgetRefExt;
        self.editor_readonly = value;
        for (widget, _, _) in &self.cells {
            widget.as_styled_input().set_is_read_only(cx, value);
        }
    }
    pub fn table_cell_at(&self, cx: &Cx, point: DVec2) -> Option<(usize, usize)> {
        if !self.editable_table || self.editor_readonly {
            return None;
        }
        let (_, range, _) = self
            .cells
            .iter()
            .find(|(widget, _, _)| widget.point_hits_area(cx, point))?;
        document_core::table::Table::parse(self.body.as_ref())
            .ok()?
            .position(range.start)
    }
    pub fn cell_focused(&self, cx: &Cx) -> bool {
        use crate::styled_input::StyledInputWidgetRefExt;
        self.cells.iter().any(|(widget, _, _)| {
            widget
                .as_styled_input()
                .borrow()
                .is_some_and(|i| i.key_focus(cx))
        })
    }
    pub fn cell_selection(&self, cx: &Cx) -> Option<std::ops::Range<usize>> {
        use crate::styled_input::StyledInputWidgetRefExt;
        self.cells.iter().find_map(|(widget, range, projection)| {
            let input = widget.as_styled_input();
            if !input.borrow().is_some_and(|i| i.key_focus(cx)) {
                return None;
            }
            let selection = input.selection();
            projection
                .source_range(selection.start().index..selection.end().index)
                .map(|r| range.start + r.start..range.start + r.end)
        })
    }
    fn draw_editable_table(&mut self, cx: &mut Cx2d) {
        use crate::styled_input::StyledInputWidgetRefExt;
        let source = self.body.as_ref().to_owned();
        let mut cell_start = 0;
        let mut header = false;
        let mut alignments = Vec::new();
        let mut column = 0;
        for (event, range) in Parser::new_ext(&source, Options::ENABLE_TABLES).into_offset_iter() {
            match event {
                MdEvent::Start(Tag::Table(values)) => {
                    self.text_flow.begin_table(cx, values.len());
                    alignments = values;
                }
                MdEvent::End(TagEnd::Table) => self.text_flow.end_table(cx),
                MdEvent::Start(Tag::TableHead) => {
                    header = true;
                    column = 0;
                    self.text_flow.begin_table_header_row(cx);
                }
                MdEvent::End(TagEnd::TableHead) => {
                    self.text_flow.end_table_row(cx);
                    header = false;
                    self.text_flow.in_table_header = false;
                }
                MdEvent::Start(Tag::TableRow) => {
                    column = 0;
                    self.text_flow.begin_table_row(cx);
                }
                MdEvent::End(TagEnd::TableRow) => self.text_flow.end_table_row(cx),
                MdEvent::Start(Tag::TableCell) => {
                    cell_start = range.start;
                    self.text_flow.begin_table_cell(
                        cx,
                        alignments.get(column).map(alignment_to_x).unwrap_or(0.0),
                    );
                }
                MdEvent::End(TagEnd::TableCell) => {
                    let raw = &source[cell_start..range.end];
                    let start = cell_start + raw.len() - raw.trim_start().len();
                    let end = range.end - raw.len() + raw.trim_end().len();
                    let range = start..end.max(start);
                    let mut projection =
                        crate::edit_projection::Projection::new(&source[range.clone()], false);
                    if projection.runs.is_empty() {
                        projection = crate::edit_projection::Projection::new("", false);
                    }
                    if header {
                        for run in &mut projection.runs {
                            run.style.bold = true;
                        }
                    }
                    self.auto_id += 1;
                    let item = self.text_flow.item(cx, LiveId(self.auto_id), id!(cell));
                    let input = item.as_styled_input();
                    input.set_is_read_only(cx, self.editor_readonly);
                    if input.text() != projection.text {
                        input.set_text(cx, &projection.text);
                    }
                    if let Some(mut native) = input.borrow_mut() {
                        native.set_runs(cx, projection.runs.clone());
                        native.set_comment_ranges(
                            cx,
                            self.comment_ranges
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
                                .collect(),
                        );
                    }
                    if self.document_selected
                        && let Some(mut native) = input.borrow_mut()
                    {
                        native.select_all(cx);
                    } else if self.had_document_selection {
                        let cursor = input.selection().cursor;
                        input.set_selection(
                            cx,
                            makepad_widgets::makepad_draw::text::selection::Selection {
                                anchor: cursor,
                                cursor,
                            },
                        );
                    }
                    item.draw_all_unscoped(cx);
                    self.cells.push((item, range, projection));
                    self.text_flow.end_table_cell(cx);
                    column += 1;
                }
                _ => {}
            }
        }
    }
    fn process_markdown_doc(&mut self, cx: &mut Cx2d) {
        let tf = &mut self.text_flow;
        // Track state for nested formatting
        let mut list_stack: Vec<ListState> = Vec::new();
        let mut is_first_block = true;
        // Per-column alignments for the current table, and the current cell's
        // column index within its row. Both are reset when a new table starts.
        let mut table_alignments: Vec<Alignment> = Vec::new();
        let mut table_cell_index: usize = 0;

        let mut events = crate::markdown_parse::events(
            self.body.as_ref(),
            Options::ENABLE_TABLES | Options::ENABLE_MATH,
        )
        .into_iter()
        .peekable();
        while let Some((event, source_range)) = events.next() {
            match event {
                MdEvent::Start(Tag::Heading { level, .. }) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    let heading_base = self.heading_base_scale;
                    let scale = match level {
                        HeadingLevel::H1 => heading_base,
                        HeadingLevel::H2 => heading_base * 0.75,
                        HeadingLevel::H3 => heading_base * 0.58,
                        HeadingLevel::H4 => heading_base * 0.5,
                        HeadingLevel::H5 => heading_base * 0.42,
                        HeadingLevel::H6 => heading_base * 0.33,
                    };
                    tf.push_size_abs_scale(scale);
                    tf.bold.push();
                    tf.font_colors.push(Vec4f::from_u32(0x054aa6ff));
                }
                MdEvent::End(TagEnd::Heading(_level)) => {
                    tf.font_colors.pop();
                    tf.bold.pop();
                    tf.font_sizes.pop();
                    tf.new_line_collapsed(cx);
                }
                MdEvent::Start(Tag::Paragraph) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                }
                MdEvent::End(TagEnd::Paragraph) => {
                    // No special handling needed, turtle position is managed by content/following blocks
                }
                MdEvent::Start(Tag::BlockQuote(_)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.begin_quote(cx);
                }
                MdEvent::End(TagEnd::BlockQuote(_quote_kind)) => {
                    tf.end_quote(cx);
                }
                MdEvent::Start(Tag::List(first_number)) => {
                    list_stack.push(ListState {
                        start_number: first_number,
                        current_number: first_number.unwrap_or(1),
                    });
                }
                MdEvent::End(TagEnd::List(_is_ordered)) => {
                    list_stack.pop();
                }
                MdEvent::Start(Tag::Item) => {
                    if !is_first_block {
                        tf.new_line_collapsed(cx);
                    }
                    is_first_block = false;
                    let marker = if let Some(state) = list_stack.last_mut() {
                        if state.start_number.is_some() {
                            // Ordered list - use and increment the counter
                            let num = state.current_number;
                            state.current_number += 1;
                            format!("{}.", num)
                        } else {
                            // Unordered list - use bullet
                            "•".to_string()
                        }
                    } else {
                        "•".to_string()
                    };
                    tf.begin_list_item(cx, &marker, 2.5);
                }
                MdEvent::End(TagEnd::Item) => {
                    tf.end_list_item(cx);
                }
                MdEvent::Start(Tag::Emphasis) => {
                    tf.italic.push();
                }
                MdEvent::End(TagEnd::Emphasis) => {
                    tf.italic.pop();
                }
                MdEvent::Start(Tag::Strong) => {
                    tf.bold.push();
                }
                MdEvent::End(TagEnd::Strong) => {
                    tf.bold.pop();
                }
                MdEvent::Start(Tag::Strikethrough) => {
                    tf.underline.push();
                }
                MdEvent::End(TagEnd::Strikethrough) => {
                    tf.underline.pop();
                }
                MdEvent::Start(Tag::Link { dest_url, .. }) => {
                    self.auto_id += 1;
                    let item = tf.item(cx, LiveId(self.auto_id), live_id!(link));
                    let mut label = String::new();
                    let mut label_runs = Vec::new();
                    for (child, range) in events.by_ref() {
                        match child {
                            MdEvent::End(TagEnd::Link) => break,
                            MdEvent::Text(t) | MdEvent::Code(t) => {
                                let start = label.len();
                                label.push_str(&t);
                                label_runs.push(crate::edit_projection::Run {
                                    visible: start..label.len(),
                                    source: range,
                                    style: Default::default(),
                                });
                            }
                            MdEvent::SoftBreak | MdEvent::HardBreak => label.push(' '),
                            _ => {}
                        }
                    }
                    item.as_doc_markdown_link().set_href(&dest_url);
                    item.set_text(cx, &label);
                    self.links.push(item.clone());
                    item.draw_all_unscoped(cx);
                    let start = tf.selection_text_len();
                    for mut run in label_runs {
                        run.visible.start += start;
                        run.visible.end += start;
                        self.source_runs.push(run);
                    }
                    tf.push_widget_text_for_selection(item.clone(), &label);
                    if self
                        .comment_ranges
                        .iter()
                        .any(|c| c.start < source_range.end && c.end > source_range.start)
                    {
                        self.comment_areas.push(item.area());
                    }
                }
                MdEvent::End(TagEnd::Link) => {
                    // Link handling is done in Start event
                }
                MdEvent::Start(Tag::Image { dest_url, .. }) => {
                    let mut alt = String::new();
                    for (child, _) in events.by_ref() {
                        match child {
                            MdEvent::End(TagEnd::Image) => break,
                            MdEvent::Text(t) => alt.push_str(&t),
                            _ => {}
                        }
                    }
                    self.auto_id += 1;
                    let id = self.auto_id;
                    let item = tf.item(cx, LiveId(id), id!(image));
                    let image = item.as_image();
                    let loaded = self
                        .images
                        .get(&id)
                        .is_some_and(|url| url == dest_url.as_ref());
                    let loaded = loaded
                        || Library::from_env()
                            .and_then(|lib| lib.read_image(&dest_url))
                            .ok()
                            .is_some_and(|data| image.load_png_from_data(cx, &data).is_ok());
                    if loaded {
                        self.images.insert(id, dest_url.to_string());
                        tf.new_line_collapsed(cx);
                        item.draw_all_unscoped(cx);
                        tf.new_line_collapsed(cx);
                    } else {
                        image.set_texture(cx, None);
                        self.images.remove(&id);
                        tf.draw_text(cx, &format!("[图片不可用：{alt}]"));
                    }
                }
                MdEvent::Start(Tag::CodeBlock(kind)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.pre_code_spacing);
                    }
                    is_first_block = false;
                    // Check if this is a runsplash block
                    let is_runsplash = matches!(&kind, CodeBlockKind::Fenced(lang) if lang.as_ref() == "runsplash");
                    if is_runsplash {
                        self.in_splash_block = true;
                        self.splash_block_string.clear();
                    } else if self.use_code_block_widget {
                        self.in_code_block = true;
                        self.code_block_string.clear();
                    } else {
                        tf.push_size_rel_scale(tf.fixed_font_size_scale);
                        tf.fixed.push();
                        tf.begin_code(cx);
                    }
                }
                MdEvent::End(TagEnd::CodeBlock) => {
                    if self.in_splash_block {
                        self.in_splash_block = false;
                        let entry_id = tf.new_counted_id();
                        let sbs = &self.splash_block_string;

                        // Draw the splash block using the $splash_block template
                        tf.item_with(cx, entry_id, id!(splash_block), |cx, item, _tf| {
                            //let tree = item.widget_tree();
                            //cx.with_vm(|vm| {
                            //    log!("$splash_block widget tree:\n{}", tree.display(vm.heap()));
                            //});
                            item.widget(cx, ids!(splash_view)).set_text(cx, sbs);
                            item.draw_all_unscoped(cx);
                        });
                    } else if self.in_code_block {
                        self.in_code_block = false;
                        let entry_id = tf.new_counted_id();
                        let cbs = &self.code_block_string;

                        // Draw the code block and capture the CodeView widget ref
                        let mut code_view_ref = WidgetRef::empty();
                        tf.item_with(cx, entry_id, id!(code_block), |cx, item, _tf| {
                            item.widget(cx, ids!(code_view)).set_text(cx, cbs);
                            item.draw_all_unscoped(cx);
                            code_view_ref = item.widget(cx, ids!(code_view));
                        });

                        // Register the code view widget for cross-child selection
                        // (its area will be queried at event time, not draw time)
                        tf.push_widget_text_for_selection(code_view_ref, &self.code_block_string);
                    } else {
                        tf.font_sizes.pop();
                        tf.fixed.pop();
                        tf.end_code(cx);
                    }
                }
                // Inline code
                MdEvent::Code(text) => {
                    tf.push_size_rel_scale(tf.fixed_font_size_scale);
                    tf.fixed.push();
                    tf.inline_code.push();
                    let raw = &self.body.as_ref()[source_range.clone()];
                    let ticks = raw.bytes().take_while(|b| *b == b'`').count();
                    draw_source_text(
                        tf,
                        cx,
                        &text,
                        source_range.start + ticks..source_range.end - ticks,
                        &self.comment_ranges,
                        &mut self.comment_areas,
                        &mut self.source_runs,
                        self.external_selection.as_ref(),
                        &mut self.selection_areas,
                    );
                    tf.font_sizes.pop();
                    tf.fixed.pop();
                    tf.inline_code.pop();
                }
                // Inline math ($...$)
                MdEvent::InlineMath(text) => {
                    if self.use_math_widget {
                        let entry_id = tf.new_counted_id();
                        tf.item_with(cx, entry_id, live_id!(inline_math), |cx, item, _tf| {
                            item.set_text(cx, &text);
                            item.draw_all_unscoped(cx);
                        });
                    } else {
                        // Fallback: render as inline code style
                        tf.push_size_rel_scale(tf.fixed_font_size_scale);
                        tf.fixed.push();
                        tf.inline_code.push();
                        tf.draw_text(cx, &text);
                        tf.font_sizes.pop();
                        tf.fixed.pop();
                        tf.inline_code.pop();
                    }
                }
                // Display math ($$...$$)
                MdEvent::DisplayMath(text) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;

                    if self.use_math_widget {
                        let entry_id = tf.new_counted_id();
                        tf.item_with(cx, entry_id, live_id!(display_math), |cx, item, _tf| {
                            item.set_text(cx, &text);
                            item.draw_all_unscoped(cx);
                        });
                    } else {
                        // Fallback: render as code block style
                        tf.begin_code(cx);
                        tf.fixed.push();
                        tf.draw_text(cx, &text);
                        tf.fixed.pop();
                        tf.end_code(cx);
                    }
                }
                MdEvent::Text(text) => {
                    if self.in_splash_block {
                        self.splash_block_string.push_str(&text);
                    } else if self.in_code_block {
                        self.code_block_string.push_str(&text);
                    } else {
                        let displayed = text.trim_end_matches('\n');
                        draw_source_text(
                            tf,
                            cx,
                            displayed,
                            source_range.clone(),
                            &self.comment_ranges,
                            &mut self.comment_areas,
                            &mut self.source_runs,
                            self.external_selection.as_ref(),
                            &mut self.selection_areas,
                        );
                    }
                }
                MdEvent::SoftBreak => {
                    if self.in_splash_block {
                        self.splash_block_string.push('\n');
                    } else if self.in_code_block {
                        self.code_block_string.push('\n');
                    } else {
                        tf.draw_text(cx, " ");
                    }
                }
                MdEvent::HardBreak => {
                    if self.in_splash_block {
                        self.splash_block_string.push('\n');
                    } else if self.in_code_block {
                        self.code_block_string.push('\n');
                    } else {
                        tf.new_line_collapsed(cx);
                    }
                }
                MdEvent::Rule => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.sep(cx);
                    tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                }
                MdEvent::TaskListMarker(_) => {
                    // TODO: Implement task list markers
                }
                MdEvent::Start(Tag::Table(alignments)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.begin_table(cx, alignments.len());
                    table_alignments = alignments;
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::Table) => {
                    tf.end_table(cx);
                    tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    table_alignments.clear();
                    table_cell_index = 0;
                }
                MdEvent::Start(Tag::TableHead) => {
                    tf.begin_table_header_row(cx);
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::TableHead) => {
                    tf.end_table_row(cx);
                    tf.in_table_header = false;
                }
                MdEvent::Start(Tag::TableRow) => {
                    tf.begin_table_row(cx);
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::TableRow) => {
                    tf.end_table_row(cx);
                }
                MdEvent::Start(Tag::TableCell) => {
                    let align_x = table_alignments
                        .get(table_cell_index)
                        .map(alignment_to_x)
                        .unwrap_or(0.0);
                    tf.begin_table_cell(cx, align_x);
                    if tf.in_table_header {
                        tf.bold.push();
                    }
                }
                MdEvent::End(TagEnd::TableCell) => {
                    if tf.in_table_header {
                        tf.bold.pop();
                    }
                    tf.end_table_cell(cx);
                    table_cell_index += 1;
                }
                MdEvent::InlineHtml(text) => {
                    // Support a handful of inline HTML tags that have no
                    // CommonMark equivalent. Anything not matched is ignored,
                    // matching the pre-existing behavior.
                    match text.trim().to_ascii_lowercase().as_str() {
                        "<u>" => {
                            tf.underline.push();
                        }
                        "</u>" => {
                            tf.underline.pop();
                        }
                        "<sub>" => {
                            tf.push_size_rel_scale(0.7);
                            tf.y_shift_scales.push(0.55);
                        }
                        "</sub>" => {
                            tf.font_sizes.pop();
                            tf.y_shift_scales.pop();
                        }
                        "<sup>" => {
                            tf.push_size_rel_scale(0.7);
                            tf.y_shift_scales.push(-0.2);
                        }
                        "</sup>" => {
                            tf.font_sizes.pop();
                            tf.y_shift_scales.pop();
                        }
                        _ => {}
                    }
                }
                _ => {} // Unimplemented or unnecessary events
            }
        }
    }
}

/// Maps pulldown_cmark table-column alignment to `Layout::align.x`.
fn alignment_to_x(alignment: &Alignment) -> f64 {
    match alignment {
        Alignment::None | Alignment::Left => 0.0,
        Alignment::Center => 0.5,
        Alignment::Right => 1.0,
    }
}

impl DocMarkdown {
    /// Preserve source activation except on a rendered link.
    pub fn link_at(&self, cx: &Cx, position: DVec2) -> bool {
        self.links
            .iter()
            .any(|link| link.point_hits_area(cx, position))
    }
}

pub fn safe_link(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

#[derive(Script, ScriptHook, Widget)]
struct DocMarkdownLink {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    link: LinkLabel,
    #[live]
    href: String,
}

impl WidgetMatchEvent for DocMarkdownLink {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        if self.link.clicked(actions)
            && let Some(url) = safe_link(&self.href)
        {
            cx.open_url(&url, OpenUrlInPlace::No);
        }
    }
}

impl Widget for DocMarkdownLink {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.link.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope)
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.link.draw_walk(cx, scope, walk)
    }

    fn text(&self) -> String {
        self.link.text()
    }

    fn set_text(&mut self, cx: &mut Cx, v: &str) {
        self.link.set_text(cx, v);
    }
}

impl DocMarkdownLinkRef {
    pub fn set_href(&self, v: &str) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.href = v.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_only_open_explicit_web_destinations() {
        assert_eq!(
            safe_link("http://www.bing.com"),
            Some("http://www.bing.com/".into())
        );
        assert!(safe_link("https://example.com/a?q=1#x").is_some());
        for value in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,x",
            "mailto:a@b.com",
            "../note.md",
            "https://user:pass@example.com",
        ] {
            assert!(safe_link(value).is_none());
        }
    }
}
