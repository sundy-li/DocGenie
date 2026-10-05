use makepad_widgets::*;
script_mod! {
    use mod.prelude.widgets.*
    let Item = ButtonFlatter{width: Fill height: 30 align: Align{x: 0.0 y: 0.5} padding: 8 grab_key_focus: false}
    mod.widgets.TabMenu = #(TabMenu::register_widget(vm)){
        width: Fill height: Fill
        body: RectView{visible: false width: 180 height: Fit flow: Down padding: 6 spacing: 2
            draw_bg +: {color: #xffffff border_color: #xdce1e8 border_size: 1.0 border_radius: 6.0}
            tab_close := Item{text: "关闭"}
            tab_close_left := Item{text: "关闭左侧"}
            tab_close_others := Item{text: "关闭其他"}
            tab_close_all := Item{text: "关闭所有"}
        }
    }
}
#[derive(Script, Widget)]
pub struct TabMenu {
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
impl ScriptHook for TabMenu {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.list = Some(DrawList2d::script_new(vm));
    }
}
impl TabMenu {
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
impl Widget for TabMenu {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        cx.begin_turtle(walk, Layout::default());
        cx.end_turtle_with_area(&mut self.area);
        if let Some(position) = self.position
            && let Some(list) = self.list.as_mut()
        {
            list.begin_overlay_reuse(cx);
            let size = cx.current_pass_size();
            cx.begin_root_turtle(size, Layout::flow_down());
            let width_estimate = 180.0_f64;
            let position = dvec2(
                position
                    .x
                    .clamp(8.0, (size.x - width_estimate - 8.0).max(8.0)),
                position.y.clamp(8.0, (size.y - 150.0).max(8.0)),
            );
            // Absolute walk keeps draw and hit-test coordinates identical.
            // ShiftTurtle tied to a reused overlay area left stale button rects.
            let _ = self.body.draw_walk(
                cx,
                &mut Scope::empty(),
                Walk {
                    abs_pos: Some(position),
                    width: Size::Fixed(180.0),
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
