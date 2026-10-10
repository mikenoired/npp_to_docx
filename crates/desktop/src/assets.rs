use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;
const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/frame.svg",
        include_bytes!("../assets/icons/frame.svg"),
    ),
    (
        "icons/arrow-right.svg",
        include_bytes!("../assets/icons/arrow-right.svg"),
    ),
    (
        "icons/book-open.svg",
        include_bytes!("../assets/icons/book-open.svg"),
    ),
    (
        "icons/check.svg",
        include_bytes!("../assets/icons/check.svg"),
    ),
    (
        "icons/chevron-down.svg",
        include_bytes!("../assets/icons/chevron-down.svg"),
    ),
    (
        "icons/chevron-right.svg",
        include_bytes!("../assets/icons/chevron-right.svg"),
    ),
    (
        "icons/circle-check.svg",
        include_bytes!("../assets/icons/circle-check.svg"),
    ),
    (
        "icons/circle-x.svg",
        include_bytes!("../assets/icons/circle-x.svg"),
    ),
    (
        "icons/close.svg",
        include_bytes!("../assets/icons/close.svg"),
    ),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    (
        "icons/external-link.svg",
        include_bytes!("../assets/icons/external-link.svg"),
    ),
    ("icons/file.svg", include_bytes!("../assets/icons/file.svg")),
    (
        "icons/folder-open.svg",
        include_bytes!("../assets/icons/folder-open.svg"),
    ),
    (
        "icons/loader-circle.svg",
        include_bytes!("../assets/icons/loader-circle.svg"),
    ),
    ("icons/moon.svg", include_bytes!("../assets/icons/moon.svg")),
    (
        "icons/search.svg",
        include_bytes!("../assets/icons/search.svg"),
    ),
    (
        "icons/settings-2.svg",
        include_bytes!("../assets/icons/settings-2.svg"),
    ),
    ("icons/sun.svg", include_bytes!("../assets/icons/sun.svg")),
    (
        "icons/triangle-alert.svg",
        include_bytes!("../assets/icons/triangle-alert.svg"),
    ),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| (*name).into())
            .collect())
    }
}
