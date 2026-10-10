use gpui::{App, Hsla, Window, px, rgb};
use gpui_component::{Theme, ThemeMode};

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Hsla,
    pub surface: Hsla,
    pub inset: Hsla,
    pub ink: Hsla,
    pub muted: Hsla,
    pub line: Hsla,
    pub accent: Hsla,
    pub selected: Hsla,
    pub good: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let color = |light, dark_color| rgb(if dark { dark_color } else { light }).into();
        Self {
            canvas: color(0xf1f4f7, 0x151e27),
            surface: color(0xffffff, 0x1c2834),
            inset: color(0xf6f8fa, 0x19232e),
            ink: color(0x152c40, 0xe0e9ef),
            muted: color(0x65788b, 0x96a8b8),
            line: color(0xdce4eb, 0x304151),
            accent: color(0x176e8a, 0x75bdd0),
            selected: color(0xe8f2f6, 0x243d4b),
            good: color(0x237859, 0x7bbda3),
            warning: color(0x976323, 0xd6ad72),
            danger: color(0xb13e45, 0xeb9399),
        }
    }
}

pub fn apply(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        Some(window),
        cx,
    );
    let p = Palette::new(dark);
    let theme = Theme::global_mut(cx);
    theme.background = p.surface;
    theme.foreground = p.ink;
    theme.border = p.line;
    theme.input = p.line;
    theme.muted = p.inset;
    theme.muted_foreground = p.muted;
    theme.accent = p.selected;
    theme.accent_foreground = p.accent;
    theme.primary = rgb(0x176e8a).into();
    theme.primary_hover = rgb(0x205f78).into();
    theme.primary_active = rgb(0x164d62).into();
    theme.primary_foreground = rgb(0xffffff).into();
    theme.secondary = p.surface;
    theme.secondary_foreground = p.ink;
    theme.secondary_hover = p.inset;
    theme.secondary_active = p.selected;
    theme.ring = p.accent;
    theme.selection = p.selected;
    theme.caret = p.accent;
    theme.font_size = px(14.0);
    theme.radius = px(6.0);
    theme.radius_lg = px(8.0);
    theme.shadow = false;
}

pub fn display_font() -> &'static str {
    if cfg!(target_os = "macos") {
        "Avenir Next"
    } else if cfg!(target_os = "windows") {
        "Bahnschrift"
    } else {
        "DejaVu Sans"
    }
}
