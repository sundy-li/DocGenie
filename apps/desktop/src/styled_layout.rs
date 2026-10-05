//! Styled glyph layout shares the native input's caret/selection geometry.
use crate::edit_projection::Run;
use makepad_widgets::makepad_draw::text::{
    color::Color,
    geom::{Point, Size},
    layouter::{LaidoutRow, LaidoutText},
};
use makepad_widgets::*;
use std::rc::Rc;

/// Structural line boxes, independent of input templates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentLayout {
    pub list: bool,
    pub markers: Vec<String>,
}
impl DocumentLayout {
    pub fn new(source: &str) -> Self {
        use pulldown_cmark::{Event, Parser, Tag};
        let mut markers = Vec::new();
        let mut lists = Vec::new();
        for event in Parser::new(source) {
            match event {
                Event::Start(Tag::List(start)) => lists.push(start),
                Event::End(pulldown_cmark::TagEnd::List(_)) => {
                    lists.pop();
                }
                Event::Start(Tag::Item) => {
                    let marker = if let Some(Some(n)) = lists.last_mut() {
                        let marker = format!("{n}.");
                        *n += 1;
                        marker
                    } else {
                        "•".to_owned()
                    };
                    markers.push(marker);
                }
                _ => {}
            }
        }
        Self {
            list: !markers.is_empty(),
            markers,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn layout(
    cx: &mut Cx,
    draw: &mut DrawText,
    bold: &TextStyle,
    italic: &TextStyle,
    fixed: &TextStyle,
    runs: &[Run],
    document: Option<&DocumentLayout>,
    text: &str,
    indent: f32,
    spacing: f32,
    width: Option<f32>,
    wrap: bool,
    align: Align,
) -> Rc<LaidoutText> {
    if runs.is_empty() || text.is_empty() {
        return draw.layout(cx, indent, spacing, width, wrap, align, text);
    }
    let base = draw.text_style.clone();
    // TextFlow align_row_height uses NORMAL font metrics at the line's size,
    // even for bold headings. Probe the same metrics for native caret/selection.
    let line_size = runs
        .iter()
        .filter_map(|r| {
            r.style
                .heading
                .map(|i| crate::typography::heading_size(i, base.font_size))
        })
        .fold(base.font_size, f32::max);
    draw.text_style.font_size = line_size;
    let normal = draw.layout(cx, 0.0, 0.0, None, false, align, "Ag");
    let normal_row = &normal.rows[0];
    draw.text_style = base.clone();
    let mut glyphs = Vec::new();
    let mut max_asc: f32 = 0.0;
    let mut min_desc: f32 = 0.0;
    let mut gap: f32 = 0.0;
    let mut cap: f32 = 0.0;
    for run in runs {
        let Some(segment) = text.get(run.visible.clone()) else {
            continue;
        };
        draw.text_style = if run.style.code {
            fixed.clone()
        } else if run.style.bold {
            bold.clone()
        } else if run.style.italic {
            italic.clone()
        } else {
            base.clone()
        };
        draw.text_style.font_size = run.style.heading.map_or(base.font_size, |i| {
            crate::typography::heading_size(i, base.font_size)
        });
        let layout = draw.layout(cx, 0.0, 0.0, None, false, align, segment);
        for row in &layout.rows {
            max_asc = max_asc.max(row.ascender_in_lpxs);
            min_desc = min_desc.min(row.descender_in_lpxs);
            gap = gap.max(row.line_gap_in_lpxs);
            cap = cap.max(row.cap_height_in_lpxs);
            for glyph in &row.glyphs {
                let mut glyph = glyph.clone();
                glyph.cluster += run.visible.start + row.text.start_in_parent();
                glyph.color = Some(Color::new(37, 48, 65, 255));
                if run.style.heading.is_some() {
                    glyph.color = Some(Color::new(5, 74, 166, 255));
                } else if run.style.link {
                    glyph.color = Some(Color::new(31, 130, 230, 255));
                }
                glyphs.push(glyph);
            }
        }
    }
    draw.text_style = base.clone();
    if glyphs.is_empty() {
        return draw.layout(cx, indent, spacing, width, wrap, align, text);
    }
    // Homogeneous prose/headings must use native word wrapping and fallback
    // metrics exactly like TextFlow, not our older glyph-by-glyph wrapper.
    if document.is_some_and(|d| !d.list) && runs.len() == 1 && runs[0].visible == (0..text.len()) {
        let run = &runs[0];
        draw.text_style = if run.style.code {
            fixed.clone()
        } else if run.style.bold {
            bold.clone()
        } else if run.style.italic {
            italic.clone()
        } else {
            base.clone()
        };
        draw.text_style.font_size = run.style.heading.map_or(base.font_size, |i| {
            crate::typography::heading_size(i, base.font_size)
        });
        let native = draw.layout(cx, indent, spacing, width, wrap, align, text);
        let mut result = (*native).clone();
        let pitch = normal_row.ascender_in_lpxs - normal_row.descender_in_lpxs
            + normal_row.ascender_in_lpxs * (base.line_spacing - 1.0);
        for (index, row) in result.rows.iter_mut().enumerate() {
            row.origin_in_lpxs.y = row.ascender_in_lpxs + index as f32 * pitch;
            for glyph in &mut row.glyphs {
                glyph.color = Some(if run.style.heading.is_some() {
                    Color::new(5, 74, 166, 255)
                } else if run.style.link {
                    Color::new(31, 130, 230, 255)
                } else {
                    Color::new(37, 48, 65, 255)
                });
            }
        }
        let last = result.rows.last().unwrap();
        result.size_in_lpxs.height = last.origin_in_lpxs.y - last.descender_in_lpxs;
        draw.text_style = base.clone();
        return Rc::new(result);
    }
    let parent: makepad_widgets::makepad_draw::text::substr::Substr = text.into();
    let mut rows = Vec::new();
    let mut start = 0;
    let marker_widths: Vec<f32> = document.map_or_else(Vec::new, |d| {
        d.markers
            .iter()
            .map(|m| {
                draw.layout(cx, 0.0, 0.0, None, false, align, m)
                    .size_in_lpxs
                    .width
                    + 5.0
                    + 3.0
            })
            .collect()
    });
    let mut item = 0;
    let mut x = indent + marker_widths.first().copied().unwrap_or(0.0);
    if document.is_some() {
        max_asc = normal_row.ascender_in_lpxs;
        min_desc = normal_row.descender_in_lpxs;
    }
    let list = document.is_some_and(|d| d.list);
    let item_inset = if list { 3.0 } else { 0.0 };
    let pitch = if list {
        max_asc - min_desc + 6.0
    } else {
        (max_asc - min_desc + gap) * base.line_spacing
    };
    let mut y = max_asc + item_inset;
    let mut row_glyphs = Vec::new();
    let mut i = 0;
    while i < glyphs.len() {
        let cluster = glyphs[i].cluster;
        let mut end = i + 1;
        while end < glyphs.len() && glyphs[end].cluster == cluster {
            end += 1;
        }
        let cluster_width: f32 = glyphs[i..end].iter().map(|g| g.advance_in_lpxs()).sum();
        // Blank lines have no glyphs but must still occupy rows, otherwise
        // full-document selection and the caret skip their visible position.
        while let Some(relative) = text[start..cluster].find('\n') {
            let stop = start + relative;
            rows.push(LaidoutRow {
                origin_in_lpxs: Point::new(0.0, y),
                text: parent.substr(start..stop),
                newline: true,
                width_in_lpxs: x,
                ascender_in_lpxs: max_asc,
                descender_in_lpxs: min_desc,
                line_gap_in_lpxs: gap,
                cap_height_in_lpxs: cap,
                line_spacing_scale: base.line_spacing,
                glyphs: std::mem::take(&mut row_glyphs),
            });
            start = stop + 1;
            item += 1;
            x = marker_widths.get(item).copied().unwrap_or(0.0);
            y += pitch;
        }
        if wrap && width.is_some_and(|w| x + cluster_width > w) && !row_glyphs.is_empty() {
            let stop = cluster;
            let newline = false;
            rows.push(LaidoutRow {
                origin_in_lpxs: Point::new(0.0, y),
                text: parent.substr(start..stop),
                newline,
                width_in_lpxs: x,
                ascender_in_lpxs: max_asc,
                descender_in_lpxs: min_desc,
                line_gap_in_lpxs: gap,
                cap_height_in_lpxs: cap,
                line_spacing_scale: base.line_spacing,
                glyphs: std::mem::take(&mut row_glyphs),
            });
            start = if newline { stop + 1 } else { stop };
            x = marker_widths.get(item).copied().unwrap_or(0.0);
            y += max_asc - min_desc + max_asc * (base.line_spacing - 1.0);
        }
        for g in &glyphs[i..end] {
            let mut g = g.clone();
            g.origin_in_lpxs = Point::new(x, 0.0);
            g.cluster -= start;
            x += g.advance_in_lpxs();
            row_glyphs.push(g);
        }
        i = end;
    }
    while let Some(relative) = text[start..].find('\n') {
        let stop = start + relative;
        rows.push(LaidoutRow {
            origin_in_lpxs: Point::new(0.0, y),
            text: parent.substr(start..stop),
            newline: true,
            width_in_lpxs: x,
            ascender_in_lpxs: max_asc,
            descender_in_lpxs: min_desc,
            line_gap_in_lpxs: gap,
            cap_height_in_lpxs: cap,
            line_spacing_scale: base.line_spacing,
            glyphs: std::mem::take(&mut row_glyphs),
        });
        start = stop + 1;
        x = 0.0;
        y += pitch;
    }
    rows.push(LaidoutRow {
        origin_in_lpxs: Point::new(0.0, y),
        text: parent.substr(start..text.len()),
        newline: false,
        width_in_lpxs: x,
        ascender_in_lpxs: max_asc,
        descender_in_lpxs: min_desc,
        line_gap_in_lpxs: gap,
        cap_height_in_lpxs: cap,
        line_spacing_scale: base.line_spacing,
        glyphs: row_glyphs,
    });
    let width = rows.iter().map(|r| r.width_in_lpxs).fold(0.0, f32::max);
    Rc::new(LaidoutText {
        text: parent,
        size_in_lpxs: Size::new(width, y - min_desc + spacing + item_inset),
        rows,
        is_truncated: false,
    })
}
