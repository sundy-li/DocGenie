//! Read-only thread summaries. All mutations remain in App / Workbench.
use document_core::Workbench;
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    mod.widgets.ThreadList = #(ThreadList::register_widget(vm)){
        width: Fill height: 168
        list := PortalList{width: Fill height: Fill flow: Down
            Thread := RectView{width: Fill height: Fit flow: Down padding: 8 spacing: 2
                draw_bg +: {color: #xffffff border_radius: 6.0 border_size: 1.0 border_color: #xe7eaee}
                thread_open := ButtonFlatter{width: Fill height: 28 align: Align{x: 0.0 y: 0.5} grab_key_focus: false
                    draw_text +: {color: #x303740 color_hover: #x2563eb text_style +: {font_size: 12}}
                }
                summary := Label{width: Fill text: "" draw_text +: {color: #x687381 text_style +: {font_size: 11}}}
            }
            Empty := View{width: Fill height: 1}
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Summary {
    id: usize,
    quote: String,
    message: String,
    stale: bool,
}
fn excerpt(text: &str, limit: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}
fn summaries(workbench: &Workbench, resolved: bool) -> Vec<Summary> {
    (0..workbench.thread_count())
        .filter_map(|id| {
            let thread = workbench.thread(id)?;
            (thread.resolved() == resolved).then(|| Summary {
                id,
                quote: excerpt(thread.original(), 22),
                message: excerpt(thread.messages().last().map_or("", |m| m.text.as_str()), 28),
                stale: thread.revision() != workbench.revision(),
            })
        })
        .collect()
}
#[derive(Script, ScriptHook, Widget)]
pub struct ThreadList {
    #[deref]
    view: View,
    #[rust]
    rows: Vec<Summary>,
    #[rust]
    active: Option<usize>,
    #[rust]
    resolved: bool,
}
impl ThreadList {
    pub fn thread_at(&self, index: usize) -> Option<usize> {
        self.rows.get(index).map(|row| row.id)
    }
    pub fn update(
        &mut self,
        cx: &mut Cx,
        workbench: &Workbench,
        resolved: bool,
        active: Option<usize>,
    ) {
        let rows = summaries(workbench, resolved);
        let height = (rows.len() as f64 * 84.0).clamp(84.0, 168.0);
        self.view.walk.height = Size::Fixed(height);
        if self.resolved != resolved {
            self.view
                .portal_list(cx, ids!(list))
                .set_first_id_and_scroll(0, 0.0);
        }
        if self.rows != rows || self.active != active || self.resolved != resolved {
            self.rows = rows;
            self.active = active;
            self.resolved = resolved;
            self.redraw(cx);
        }
    }
}
impl Widget for ThreadList {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.rows.len());
                while let Some(index) = list.next_visible_item(cx) {
                    let mut row = list.item(
                        cx,
                        index,
                        if index < self.rows.len() {
                            id!(Thread)
                        } else {
                            id!(Empty)
                        },
                    );
                    if let Some(summary) = self.rows.get(index) {
                        row.button(cx, ids!(thread_open))
                            .set_text(cx, &summary.quote);
                        row.label(cx, ids!(summary)).set_text(
                            cx,
                            &format!(
                                "{}{}",
                                if summary.stale {
                                    "原文已变化 · "
                                } else {
                                    ""
                                },
                                summary.message
                            ),
                        );
                        let color = Vec4f::from_u32(if self.active == Some(summary.id) {
                            0xeff4fcff
                        } else {
                            0xffffffff
                        });
                        script_apply_eval!(cx, row, {draw_bg +: {color: #(color)}});
                    }
                    row.draw_all_unscoped(cx);
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_preserve_ids_and_mark_stale_anchors() {
        let mut wb = Workbench::default();
        wb.set_text("相同文字\n\n相同文字".into()).unwrap();
        let first = wb.comment(0..12, "第一条").unwrap();
        let second = wb.comment(14..26, "第二条").unwrap();
        wb.resolve_thread(first).unwrap();
        assert_eq!(summaries(&wb, false)[0].id, second);
        assert_eq!(summaries(&wb, true)[0].id, first);
        wb.reopen_thread(first).unwrap();
        assert_eq!(summaries(&wb, false).len(), 2);
        wb.set_text("完全变化".into()).unwrap();
        assert!(summaries(&wb, false).iter().all(|s| s.stale));
    }
    #[test]
    fn excerpts_are_bounded_unicode_and_collapse_whitespace() {
        assert_eq!(excerpt(" 中文\n 😀 e\u{301} ", 4), "中文 😀…");
        assert_eq!(excerpt("\n 相同  内容 \n", 20), "相同 内容");
    }
}
