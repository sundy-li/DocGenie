use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    let FormatButton = Button{width: Fit height: 28 padding: Inset{left: 9 right: 9 top: 4 bottom: 4} grab_key_focus: false
        draw_text +: {color: #x303740 color_hover: #x2162c2 color_down: #x2162c2 text_style +: {font_size: 13}}
    }
    mod.widgets.CommentMenu = #(CommentMenu::register_widget(vm)){
        width: Fill height: Fill
        body: RectView{visible: false width: Fit height: Fit flow: Down padding: 6 spacing: 2
            draw_bg +: {color: #xffffff border_color: #xdce1e8 border_size: 1.0 border_radius: 8.0}
            format_row := View{width: Fit height: Fit flow: Right spacing: 0
                fmt_bold := FormatButton{text: "B"}
                fmt_italic := FormatButton{text: "I"}
                fmt_underline := FormatButton{text: "U"}
                fmt_code := FormatButton{text: "</>"}
                fmt_link := FormatButton{text: "🔗"}
            }
            action_row := View{width: Fit height: Fit flow: Right spacing: 0
                add_comment := Button{text: "💬 评论" width: Fit height: 32 padding: Inset{left: 10 right: 10 top: 6 bottom: 6} grab_key_focus: false
                    draw_text +: {color: #x303740 color_hover: #x2162c2 color_down: #x2162c2}
                }
                toolbar_agent := Button{text: "✨ Agent 完善" visible: false width: Fit height: 32 padding: Inset{left: 10 right: 10 top: 6 bottom: 6} grab_key_focus: false
                    draw_text +: {color: #x303740 color_hover: #x2162c2 color_down: #x2162c2}
                }
            }
        }
    }
    mod.widgets.CommentComposer = #(CommentComposer::register_widget(vm)){
        width: Fill height: Fill
        body: RectView{visible: false width: Fit height: Fit flow: Down padding: 12 spacing: 8
            draw_bg +: {color: #xffffff border_color: #xdce1e8 border_size: 1.0 border_radius: 8.0}
            composer_quote := TextInput{width: 320 height: Fit is_multiline: true is_read_only: true empty_text: ""
                draw_bg +: {color: #xfff7d6 color_empty: #xfff7d6 color_hover: #fff7d6 color_focus: #fff7d6 border_color_top: #xffc107 border_size_top: 3.0}
            }
            composer_input := TextInput{width: 320 height: 72 is_multiline: true empty_text: "添加评论…" blink_speed: 0.5
                draw_bg +: {color: #xffffff color_empty: #xffffff color_hover: #xffffff color_focus: #xffffff border_color: #xdce1e8 border_color_focus: #x2563eb}
                draw_cursor +: {color: #x2563eb}
            }
            View{width: Fit height: Fit flow: Right spacing: 8
                composer_ask_ai := CheckBox{text: "✨ 问 AI"}
                composer_cancel := Button{text: "取消" width: Fit height: 30 padding: Inset{left: 12 right: 12 top: 5 bottom: 5}
                    draw_text +: {color: #x303740}
                }
                composer_submit := Button{text: "评论" width: Fit height: 30 padding: Inset{left: 14 right: 14 top: 5 bottom: 5}
                    draw_bg +: {color: #x2563eb color_hover: #x1d4ed8 color_down: #x1d4ed8}
                    draw_text +: {color: #xffffff}
                }
            }
            composer_hint := Label{text: "「问 AI」会外发选区和评论；默认自动修改见 Preferences" draw_text +: {color: #x8a919c text_style +: {font_size: 11}}}
        }
    }
}
#[derive(Script, Widget)]
pub struct CommentMenu {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[live]
    #[find]
    #[redraw]
    body: WidgetRef,
    #[rust]
    list: Option<DrawList2d>,
    #[rust]
    position: Option<DVec2>,
    #[rust]
    #[visible]
    visible: bool,
    #[rust]
    area: Area,
}
impl ScriptHook for CommentMenu {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.list = Some(DrawList2d::script_new(vm));
    }
}
impl CommentMenu {
    pub fn contains(&self, cx: &Cx, point: DVec2) -> bool {
        self.position.is_some() && self.body.point_hits_area(cx, point)
    }
    pub fn show(&mut self, cx: &mut Cx, position: DVec2) {
        self.position = Some(position);
        self.visible = true;
        self.body.set_visible(cx, true);
        self.redraw(cx);
        cx.redraw_all();
    }
    pub fn hide(&mut self, cx: &mut Cx) {
        if self.position.is_none() {
            return;
        }
        self.position = None;
        self.visible = false;
        self.body.set_visible(cx, false);
        self.redraw(cx);
        cx.redraw_all();
    }
}
impl CommentMenu {
    pub fn set_agent_visible(&self, cx: &mut Cx, visible: bool) {
        self.body
            .button(cx, ids!(toolbar_agent))
            .set_visible(cx, visible);
    }
    /// Markdown style buttons only make sense while editing; reading mode
    /// keeps the comment/agent actions alone.
    pub fn set_format_visible(&self, cx: &mut Cx, visible: bool) {
        self.body
            .view(cx, ids!(format_row))
            .set_visible(cx, visible);
    }
}
impl Widget for CommentMenu {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        cx.begin_turtle(walk, Layout::default());
        cx.end_turtle_with_area(&mut self.area);
        if let Some(position) = self.position
            && let Some(list) = self.list.as_mut()
        {
            list.begin_overlay_reuse(cx);
            let size = cx.current_pass_size();
            cx.begin_root_turtle(size, Layout::flow_down());
            let width_estimate = 300.0_f64;
            let position = dvec2(
                position
                    .x
                    .clamp(8.0, (size.x - width_estimate - 8.0).max(8.0)),
                position.y.clamp(8.0, (size.y - 100.0).max(8.0)),
            );
            // Absolute walk keeps draw and hit-test coordinates identical.
            // ShiftTurtle tied to a reused overlay area left stale button rects.
            let _ = self.body.draw_walk(
                cx,
                &mut Scope::empty(),
                Walk {
                    abs_pos: Some(position),
                    width: Size::Fixed(236.0),
                    height: Size::fit(),
                    ..Walk::default()
                },
            );
            cx.end_turtle();
            list.end(cx);
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.position.is_some() {
            // Deliberately NOT hidden on Scroll: macOS trackpads keep emitting
            // inertial scroll events after the fingers lift, which used to
            // dismiss the toolbar the instant a post-scroll selection raised
            // it. The selection is range-based and stays valid while
            // scrolling, so the toolbar stays too.
            if matches!(event, Event::KeyDown(key) if !crate::live_editor::select_all_key(key)) {
                self.hide(cx);
                return;
            }
            if let Event::MouseDown(mouse) = event {
                let inside = self.contains(cx, mouse.abs);
                if !inside {
                    self.hide(cx);
                    return;
                }
            }
            self.body.handle_event(cx, event, scope);
        }
    }
}

/// Inline comment card anchored at the selection: quote, input and a
/// per-comment 「问 AI」 switch. Kept separate from the sidebar so the
/// document flow stays Feishu-style; the sidebar lists existing threads.
#[derive(Script, Widget)]
pub struct CommentComposer {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[live]
    #[find]
    #[redraw]
    body: WidgetRef,
    #[rust]
    list: Option<DrawList2d>,
    #[rust]
    position: Option<DVec2>,
    #[rust]
    #[visible]
    visible: bool,
    #[rust]
    area: Area,
}
impl ScriptHook for CommentComposer {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.list = Some(DrawList2d::script_new(vm));
    }
}
impl CommentComposer {
    pub fn contains(&self, cx: &Cx, point: DVec2) -> bool {
        self.position.is_some() && self.body.point_hits_area(cx, point)
    }
    pub fn is_open(&self) -> bool {
        self.position.is_some()
    }
    pub fn show(&mut self, cx: &mut Cx, position: DVec2, quote: &str, ask_ai: bool) {
        self.position = Some(position);
        self.visible = true;
        self.body.set_visible(cx, true);
        let quote = quote.trim();
        let quote = if quote.chars().count() > 120 {
            format!("{}…", quote.chars().take(120).collect::<String>())
        } else {
            quote.to_owned()
        };
        self.body
            .text_input(cx, ids!(composer_quote))
            .set_text(cx, &quote);
        self.body
            .text_input(cx, ids!(composer_input))
            .set_text(cx, "");
        self.body
            .check_box(cx, ids!(composer_ask_ai))
            .set_active(cx, ask_ai, Animate::No);
        self.body.label(cx, ids!(composer_hint)).set_text(
            cx,
            if ask_ai {
                "默认自动修改已开启：提交后会外发选区和评论"
            } else {
                "勾选「问 AI」会外发选区和评论，由 Agent 自动修改"
            },
        );
        self.body
            .button(cx, ids!(composer_submit))
            .set_enabled(cx, false);
        self.redraw(cx);
        cx.redraw_all();
        self.body
            .text_input(cx, ids!(composer_input))
            .take_key_focus(cx);
    }
    pub fn hide(&mut self, cx: &mut Cx) {
        if self.position.is_none() {
            return;
        }
        self.position = None;
        self.visible = false;
        self.body.set_visible(cx, false);
        self.redraw(cx);
        cx.redraw_all();
    }
}
impl Widget for CommentComposer {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        cx.begin_turtle(walk, Layout::default());
        cx.end_turtle_with_area(&mut self.area);
        if let Some(position) = self.position
            && let Some(list) = self.list.as_mut()
        {
            list.begin_overlay_reuse(cx);
            let size = cx.current_pass_size();
            cx.begin_root_turtle(size, Layout::flow_down());
            let width_estimate = 360.0_f64;
            let position = dvec2(
                position
                    .x
                    .clamp(8.0, (size.x - width_estimate - 8.0).max(8.0)),
                position.y.clamp(8.0, (size.y - 280.0).max(8.0)),
            );
            let _ = self.body.draw_walk(
                cx,
                &mut Scope::empty(),
                Walk {
                    abs_pos: Some(position),
                    width: Size::Fixed(348.0),
                    height: Size::fit(),
                    ..Walk::default()
                },
            );
            cx.end_turtle();
            list.end(cx);
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.position.is_some() {
            if let Event::Actions(actions) = event
                && let Some(text) = self
                    .body
                    .text_input(cx, ids!(composer_input))
                    .changed(actions)
            {
                self.body
                    .button(cx, ids!(composer_submit))
                    .set_enabled(cx, !text.trim().is_empty());
            }
            // The composer holds a text field: keys must flow through, only an
            // outside press dismisses the card.
            if let Event::MouseDown(mouse) = event
                && mouse.button == MouseButton::PRIMARY
                && !self.contains(cx, mouse.abs)
            {
                self.hide(cx);
                return;
            }
            self.body.handle_event(cx, event, scope);
        }
    }
}
