//! User-selectable code font for the document renderers.
//!
//! The Preferences panel keeps an ordered priority list of installed font
//! family names; the first family that resolves to a loadable face replaces
//! the theme's bundled `font_code` family at runtime. An empty list — or one
//! where nothing resolves — restores the bundled font. The scan and the
//! loadable-face checks follow the pinned Makepad terminal's `fonts.rs`.
//!
//! Mechanism: `theme.font_code`'s family is a single-member chain
//! (Liberation Mono, no lazy members in either font-set policy), so its
//! loader definition can be swapped for the user's face without the widgets'
//! `ensure_fonts_loaded` ever fighting back — they only re-define a family
//! whose definition looks incomplete, which is exactly how the bundled font
//! is restored (see `restore_bundled`). System fallback faces are appended
//! by the loader behind every family, so CJK/emoji coverage is unaffected.
use makepad_widgets::makepad_draw::{
    cx_draw::CxDraw,
    makepad_platform::{SharedBytes, thread::SignalToUI},
    text::{
        font::FontId,
        font_family::{FontDiagnostics, FontFamilyId},
        fonts::Fonts,
        loader::{FontDefinition, FontFamilyDefinition},
        system_fonts::{Family, face_loadable, font_dirs, group, scan},
    },
};
use makepad_widgets::*;
use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::OnceLock,
};

static FAMILIES: OnceLock<Vec<Family>> = OnceLock::new();
static SCANNING: std::sync::Once = std::sync::Once::new();

/// Start the installed-font scan on a thread (about a second on macOS, where
/// fonts downloaded on demand live in many directories). When it finishes a
/// UI signal fires so the app can re-apply the configured priority list.
pub fn warm_scan() {
    SCANNING.call_once(|| {
        let _ = std::thread::Builder::new()
            .name("docgenie-font-scan".into())
            .spawn(|| {
                FAMILIES.get_or_init(scan_usable);
                SignalToUI::set_ui_signal();
            });
    });
}

/// The installed families whose regular face the text engine can load.
fn scan_usable() -> Vec<Family> {
    let mut families = group(scan(&font_dirs()));
    families.retain(|family| face_loadable(&family.regular));
    families
}

/// Whether the background scan has finished.
pub fn ready() -> bool {
    FAMILIES.get().is_some()
}

/// Family names for the add-font menu: monospace first, then alphabetical.
/// Empty until the scan `warm_scan` started has finished.
pub fn menu_names() -> Vec<String> {
    let Some(families) = FAMILIES.get() else {
        return Vec::new();
    };
    let mut names: Vec<String> = families.iter().map(|family| family.name.clone()).collect();
    names.sort_by(|a, b| {
        let mono = |name: &str| {
            !families
                .iter()
                .any(|family| family.name == name && family.monospace)
        };
        mono(a).cmp(&mono(b)).then_with(|| a.cmp(b))
    });
    names
}

fn find(name: &str) -> Option<&'static Family> {
    FAMILIES
        .get()?
        .iter()
        .find(|family| family.name.eq_ignore_ascii_case(name))
}

/// `theme.font_code`'s loader family id, read through the script VM so the
/// content-hash internals stay private to the draw crate.
fn theme_code_family_id(cx: &mut Cx) -> Option<FontFamilyId> {
    cx.with_vm(|vm| {
        let value = script_eval!(vm, { mod.theme.font_code });
        if value.is_nil() {
            return None;
        }
        Some(TextStyle::script_from_value(vm, value).font_family_id())
    })
}

/// Apply the priority list to the theme's code font family. Returns the
/// family name now in effect, or `None` when the bundled font is in effect
/// (empty/unresolvable list, or the scan has not finished yet — the caller
/// re-applies when the scan-ready signal arrives).
pub fn apply_code_font(cx: &mut Cx, priority: &[String]) -> Option<String> {
    if !ready() {
        return None;
    }
    CxDraw::lazy_construct_fonts(cx);
    let family_id = theme_code_family_id(cx)?;
    let fonts = cx.get_global::<Rc<RefCell<Fonts>>>().clone();
    let resolved = priority.iter().find_map(|name| {
        let family = find(name)?;
        face_loadable(&family.regular).then(|| (name.clone(), family.regular.clone()))
    });
    let mut fonts = fonts.borrow_mut();
    match resolved {
        Some((name, face)) => {
            let mut hasher = DefaultHasher::new();
            ("docgenie-code-font", &face.path, face.index).hash(&mut hasher);
            let font_id = FontId::from(hasher.finish());
            if !fonts.is_font_known(font_id) {
                let Ok(data) = SharedBytes::from_file_mmap_or_read(&face.path) else {
                    return None;
                };
                fonts.define_font(
                    font_id,
                    FontDefinition {
                        data,
                        index: face.index,
                        ascender_fudge_in_ems: 0.0,
                        descender_fudge_in_ems: 0.0,
                        weight: None,
                        variations: Vec::new(),
                    },
                );
            }
            fonts.set_font_family_definition(
                family_id,
                FontFamilyDefinition {
                    font_ids: vec![font_id],
                    expected_member_count: 1,
                    diagnostics: FontDiagnostics {
                        role: "code".to_owned(),
                        set: "user".to_owned(),
                        tried: vec![name.clone()],
                    },
                },
            );
            Some(name)
        }
        None => {
            restore_bundled(&mut fonts, family_id);
            None
        }
    }
}

/// Mark the family incomplete so the next widget draw re-installs the bundled
/// theme definition (`ensure_fonts_loaded` self-heals any family whose stored
/// definition does not match what its members imply).
fn restore_bundled(fonts: &mut Fonts, family_id: FontFamilyId) {
    fonts.set_font_family_definition(
        family_id,
        FontFamilyDefinition {
            font_ids: Vec::new(),
            expected_member_count: usize::MAX,
            diagnostics: FontDiagnostics::default(),
        },
    );
}
