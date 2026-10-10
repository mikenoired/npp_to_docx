use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use encoding_rs::{Encoding, UTF_8, WINDOWS_1251};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};
use regex::{Captures, Regex};
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::Path,
    sync::LazyLock,
};

static ENCODING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)encoding\s*=\s*["']([^"']+)["']"#).unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-?\d+(\.\d+)?").unwrap());
static IMAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<image\b[^>]*>").unwrap());
static HREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:xlink:href|href)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap()
});
static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)&#(?:x([0-9a-f]+)|(\d+));").unwrap());
static DYN_TYPE: LazyLock<Regex> = LazyLock::new(|| Regex::new("[^a-zA-Z0-9]+").unwrap());
static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z]+:").unwrap());
static SVG_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<svg\b").unwrap());
const PLACEHOLDER: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\" viewBox=\"0 0 100 100\"><rect x=\"1\" y=\"1\" width=\"98\" height=\"98\" fill=\"#fff4e5\" stroke=\"#b66a00\" stroke-width=\"2\"/><path d=\"M5 5L95 95M95 5L5 95\" stroke=\"#b66a00\" stroke-width=\"2\"/></svg>";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub index: usize,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submodel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_mismatch: bool,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point: Option<Point>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    pub markers: Vec<Marker>,
    pub view_width: Option<f64>,
    pub view_height: Option<f64>,
}

pub fn decode(bytes: &[u8]) -> (String, String) {
    let prolog: String = bytes.iter().take(512).map(|b| char::from(*b)).collect();
    let requested = ENCODING
        .captures(&prolog)
        .map(|c| c[1].to_lowercase())
        .unwrap_or("utf-8".into());
    let encoding = Encoding::for_label(requested.as_bytes()).unwrap_or(UTF_8);
    let encoding = if encoding == UTF_8 && std::str::from_utf8(bytes).is_err() {
        WINDOWS_1251
    } else {
        encoding
    };
    let (text, _, _) = encoding.decode(bytes);
    (
        text.trim_start_matches('\u{feff}').into(),
        encoding.name().to_lowercase(),
    )
}

pub fn utf8_xml(content: &str) -> String {
    let mut value = ENCODING.replace(content, "encoding=\"UTF-8\"").into_owned();
    value = ENTITY
        .replace_all(&value, |c: &Captures| {
            let code = if let Some(h) = c.get(1) {
                u32::from_str_radix(h.as_str(), 16).unwrap_or(0)
            } else {
                c[2].parse().unwrap_or(0)
            };
            if matches!(code,9|10|13|32..=0xd7ff|0xe000..=0xfffd|0x10000..=0x10ffff) {
                c[0].to_string()
            } else {
                String::new()
            }
        })
        .into_owned();
    for (prefix, uri) in [
        ("rt", "http://www.rts.co.at/2001/XMLSchema/RtSvgPL"),
        ("xlink", "http://www.w3.org/1999/xlink"),
    ] {
        if !value.contains(&format!("xmlns:{prefix}")) {
            value = SVG_TAG
                .replace(&value, format!("<svg xmlns:{prefix}=\"{uri}\""))
                .into_owned();
        }
    }
    value
}

fn clean(raw: &str) -> String {
    let unescaped = quick_xml::escape::unescape(raw)
        .map(|s| s.into_owned())
        .unwrap_or(raw.into());
    crate::db::normalize(&unescaped)
}

fn nonempty(raw: &str) -> Option<String> {
    let value = clean(raw);
    if value.is_empty() { None } else { Some(value) }
}

fn number(value: Option<&String>) -> Option<f64> {
    NUMBER.find(value?).and_then(|n| n.as_str().parse().ok())
}

#[derive(Default)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    dyn_values: HashMap<String, String>,
    title: Option<String>,
    text: String,
    child_point: Option<Point>,
}

fn point(node: &Node) -> Option<Point> {
    let get = |names: &[&str]| names.iter().find_map(|name| node.attrs.get(*name));
    let x = number(get(&["x", "cx", "x1"]));
    let y = number(get(&["y", "cy", "y1"]));
    if let (Some(mut x), Some(y)) = (x, y) {
        if matches!(node.name.as_str(), "image" | "rect")
            && number(get(&["height"])).is_some()
            && let Some(width) = number(get(&["width"]))
        {
            x += width;
        }
        return Some(Point { x, y });
    }
    for pair in node.attrs.get("points")?.split_whitespace() {
        if let Some((x, y)) = pair.split_once(',')
            && let (Some(x), Some(y)) = (number(Some(&x.into())), number(Some(&y.into())))
        {
            return Some(Point { x, y });
        }
    }
    None
}

fn kks(node: &Node) -> Option<String> {
    ["FKKS", "KKS", "POINTID_STATUS", "POINTID"]
        .iter()
        .find_map(|key| node.dyn_values.get(*key).cloned())
}

fn open(tag: &BytesStart, stack: &mut Vec<Node>, parsed: &mut Parsed) -> Result<()> {
    let name = String::from_utf8_lossy(tag.name().as_ref()).to_lowercase();
    let mut attrs = HashMap::new();
    for attr in tag.attributes().with_checks(false) {
        let attr = attr?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
        let value = String::from_utf8_lossy(&attr.value);
        let value = quick_xml::escape::unescape(&value)
            .map(|s| s.into_owned())
            .unwrap_or(value.into());
        attrs.insert(key, value);
    }
    if name == "rt:dyn"
        && let Some(parent) = stack.last_mut()
        && let (Some(kind), Some(value)) = (attrs.get("type"), attrs.get("value"))
        && let Some(value) = nonempty(value)
    {
        let kind = DYN_TYPE
            .replace_all(&clean(kind), "_")
            .trim_matches('_')
            .to_uppercase();
        parent.dyn_values.insert(kind, value);
    }
    if name == "svg" && (parsed.view_width.is_none() || parsed.view_height.is_none()) {
        let view: Vec<f64> = attrs
            .get("viewbox")
            .map(|v| {
                v.split(|c: char| c.is_whitespace() || c == ',')
                    .filter_map(|n| n.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        parsed.view_width = view.get(2).copied().or_else(|| number(attrs.get("width")));
        parsed.view_height = view.get(3).copied().or_else(|| number(attrs.get("height")));
    }
    stack.push(Node {
        name,
        attrs,
        ..Default::default()
    });
    Ok(())
}

fn close(stack: &mut Vec<Node>, parsed: &mut Parsed) {
    let Some(node) = stack.pop() else { return };
    if node.name == "title" {
        if let Some(parent) = stack.last_mut()
            && parent.title.is_none()
        {
            parent.title = Some(node.text);
        }
        return;
    }
    let own_point = point(&node);
    if let Some(parent) = stack.last_mut()
        && parent.child_point.is_none()
    {
        parent.child_point = own_point;
    }
    let dyn_kks = kks(&node);
    if node.name != "svg" && (node.title.is_some() || dyn_kks.is_some()) {
        let title = clean(node.title.as_deref().or(dyn_kks.as_deref()).unwrap_or(""));
        let title = if title.is_empty() {
            "(empty title)".into()
        } else {
            title
        };
        let kks = dyn_kks.or_else(|| {
            if title == "(empty title)" {
                None
            } else {
                Some(title.clone())
            }
        });
        parsed.markers.push(Marker {
            index: parsed.markers.len() + 1,
            is_mismatch: title != "(empty title)" && kks.as_ref().is_some_and(|k| *k != title),
            title,
            kks,
            submodel: node
                .attrs
                .get("xlink:href")
                .or(node.attrs.get("href"))
                .and_then(|v| nonempty(v)),
            description: None,
            tag: node.name,
            point: own_point.or(node.child_point),
        });
    }
}

pub fn parse(content: &str) -> Result<Parsed> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().check_end_names = false;
    let mut stack = Vec::new();
    let mut parsed = Parsed::default();
    loop {
        match reader.read_event()? {
            Event::Start(tag) => open(&tag, &mut stack, &mut parsed)?,
            Event::Empty(tag) => {
                open(&tag, &mut stack, &mut parsed)?;
                close(&mut stack, &mut parsed);
            }
            Event::End(_) => close(&mut stack, &mut parsed),
            Event::Text(text) => {
                if let Some(node) = stack.last_mut()
                    && node.name == "title"
                {
                    node.text.push_str(&text.decode()?);
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(node) = stack.last_mut()
                    && node.name == "title"
                {
                    node.text.push('&');
                    node.text.push_str(&reference.decode()?);
                    node.text.push(';');
                }
            }
            Event::CData(text) => {
                if let Some(node) = stack.last_mut()
                    && node.name == "title"
                {
                    node.text.push_str(&text.decode()?);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(parsed)
}

pub fn prepare_resources(content: &str, path: &Path) -> (String, Vec<String>) {
    let mut missing = BTreeSet::new();
    let svg = IMAGE
        .replace_all(content, |tag: &Captures| {
            let Some(href) = HREF.captures(&tag[0]) else {
                return tag[0].into();
            };
            let reference = href.get(1).or(href.get(2)).unwrap().as_str();
            if reference.starts_with("data:") {
                return tag[0].into();
            };
            let name = reference.replace('\\', "/");
            let safe = !Path::new(&name).is_absolute()
                && !name.split('/').any(|v| v == "..")
                && !SCHEME.is_match(&name);
            let data = if safe {
                fs::read(path.parent().unwrap_or(Path::new(".")).join(name)).ok()
            } else {
                None
            };
            let lower = reference.to_lowercase();
            let mut uri = None;
            if let Some(data) = data {
                if lower.ends_with(".svg") {
                    let text = utf8_xml(&decode(&data).0);
                    let text = normalize_paints(&text).unwrap_or(text);
                    let external = IMAGE.captures_iter(&text).any(|image| {
                        HREF.captures(&image[0]).is_some_and(|h| {
                            !h.get(1).or(h.get(2)).unwrap().as_str().starts_with("data:")
                        })
                    });
                    if !external {
                        uri = Some(format!(
                            "data:image/svg+xml;base64,{}",
                            STANDARD.encode(text)
                        ));
                    }
                } else if lower.ends_with(".png")
                    || lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                {
                    uri = Some(format!(
                        "data:image/{};base64,{}",
                        if lower.ends_with(".png") {
                            "png"
                        } else {
                            "jpeg"
                        },
                        STANDARD.encode(data)
                    ));
                }
            }
            let uri = uri.unwrap_or_else(|| {
                missing.insert(reference.to_string());
                format!("data:image/svg+xml;base64,{}", STANDARD.encode(PLACEHOLDER))
            });
            HREF.replace(
                &tag[0],
                format!("{}=\"{uri}\"", href[0].split('=').next().unwrap()),
            )
            .into_owned()
        })
        .into_owned();
    (svg, missing.into_iter().collect())
}

pub struct Renderer {
    options: usvg::Options<'static>,
}

// RtSvg exports symbolic palette names such as Col_Grey2 as SVG paint values.
// librsvg ignores those invalid declarations and inherits the parent's paint;
// usvg instead substitutes black. Remove invalid presentation attributes so
// diagrams retain the reference renderer's inheritance behavior.
fn normalize_paints(content: &str) -> Result<String> {
    let mut reader = Reader::from_str(content);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader.read_event()? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let (tag, empty) = match event {
                    Event::Start(tag) => (tag, false),
                    Event::Empty(tag) => (tag, true),
                    _ => unreachable!(),
                };
                let mut normalized = BytesStart::from_content(
                    String::from_utf8_lossy(tag.as_ref()).into_owned(),
                    tag.name().as_ref().len(),
                );
                normalized.clear_attributes();
                for attr in tag.attributes().with_checks(false) {
                    let attr = attr?;
                    let raw = String::from_utf8_lossy(&attr.value);
                    let value = quick_xml::escape::unescape(&raw).unwrap_or_else(|_| raw.clone());
                    let key = attr.key.as_ref();
                    let wide = matches!(
                        value.as_ref(),
                        "inherit"
                            | "initial"
                            | "unset"
                            | "revert"
                            | "revert-layer"
                            | "currentColor"
                    );
                    let invalid = !wide
                        && if key.eq_ignore_ascii_case(b"fill")
                            || key.eq_ignore_ascii_case(b"stroke")
                        {
                            svgtypes::Paint::from_str(&value).is_err()
                        } else if key.eq_ignore_ascii_case(b"color") {
                            value.parse::<svgtypes::Color>().is_err()
                        } else {
                            false
                        };
                    if !invalid {
                        normalized.push_attribute(attr);
                    }
                }
                writer.write_event(if empty {
                    Event::Empty(normalized)
                } else {
                    Event::Start(normalized)
                })?;
            }
            Event::Eof => break,
            event => writer.write_event(event)?,
        }
    }
    Ok(String::from_utf8(writer.into_inner())?)
}

// resvg rasterizes each SVG image into a full-size intermediate surface. The
// repeated missing-resource symbol is equivalent to a nested vector viewport,
// so inline it rather than allocating a surface for every absent submodel.
fn inline_placeholders(content: &str) -> Result<String> {
    // Renaming image to svg could change type selectors in a stylesheet.
    // Retain the original representation whenever a stylesheet is present.
    if content.to_ascii_lowercase().contains("<style") {
        return Ok(content.to_string());
    }
    let uri = format!("data:image/svg+xml;base64,{}", STANDARD.encode(PLACEHOLDER));
    let shapes = PLACEHOLDER
        .split_once('>')
        .unwrap()
        .1
        .strip_suffix("</svg>")
        .unwrap();
    let mut reader = Reader::from_str(content);
    let mut writer = Writer::new(Vec::new());
    let mut replaced = Vec::new();
    loop {
        match reader.read_event()? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let (tag, empty) = match event {
                    Event::Start(tag) => (tag, false),
                    Event::Empty(tag) => (tag, true),
                    _ => unreachable!(),
                };
                let has_transform = tag
                    .attributes()
                    .with_checks(false)
                    .filter_map(|a| a.ok())
                    .any(|a| {
                        a.key.as_ref().eq_ignore_ascii_case(b"transform")
                            || (a.key.as_ref().eq_ignore_ascii_case(b"style")
                                && String::from_utf8_lossy(&a.value).contains("transform"))
                    });
                let is_placeholder = !has_transform
                    && tag.name().as_ref().eq_ignore_ascii_case(b"image")
                    && tag
                        .attributes()
                        .with_checks(false)
                        .filter_map(|a| a.ok())
                        .any(|a| {
                            (a.key.as_ref().eq_ignore_ascii_case(b"href")
                                || a.key.as_ref().eq_ignore_ascii_case(b"xlink:href"))
                                && a.value.as_ref() == uri.as_bytes()
                        });
                if is_placeholder {
                    let mut viewport = BytesStart::new("svg");
                    for attr in tag.attributes().with_checks(false) {
                        let attr = attr?;
                        if ![b"href".as_slice(), b"xlink:href", b"viewBox"]
                            .iter()
                            .any(|key| attr.key.as_ref().eq_ignore_ascii_case(key))
                        {
                            viewport.push_attribute(attr);
                        }
                    }
                    viewport.push_attribute(("viewBox", "0 0 100 100"));
                    writer.write_event(Event::Start(viewport))?;
                    writer.get_mut().extend_from_slice(shapes.as_bytes());
                    if empty {
                        writer.write_event(Event::End(BytesEnd::new("svg")))?;
                    } else {
                        replaced.push(true);
                    }
                } else if empty {
                    writer.write_event(Event::Empty(tag))?;
                } else {
                    writer.write_event(Event::Start(tag))?;
                    replaced.push(false);
                }
            }
            Event::End(tag) => {
                if replaced.pop() == Some(true) {
                    writer.write_event(Event::End(BytesEnd::new("svg")))?;
                } else {
                    writer.write_event(Event::End(tag))?;
                }
            }
            Event::Eof => break,
            event => writer.write_event(event)?,
        }
    }
    Ok(String::from_utf8(writer.into_inner())?)
}

// Keep only fonts actually selected by a document in memory. File-backed
// fontdb faces otherwise reopen and mmap the font for every glyph outline.
// The shared database grows additively, so IDs in existing trees remain valid.
struct FontCache {
    database: std::sync::Arc<usvg::fontdb::Database>,
    selected: std::collections::HashMap<usvg::Font, Option<usvg::fontdb::ID>>,
    resident: std::collections::HashMap<usvg::fontdb::ID, usvg::fontdb::ID>,
}

impl FontCache {
    fn make_resident(&mut self, id: usvg::fontdb::ID) -> usvg::fontdb::ID {
        if let Some(resident) = self.resident.get(&id) {
            return *resident;
        }
        let Some(mut face) = self.database.face(id).cloned() else {
            return id;
        };
        let usvg::fontdb::Source::File(path) = &face.source else {
            return id;
        };
        let Ok(bytes) = std::fs::read(path) else {
            return id;
        };
        face.source = usvg::fontdb::Source::Binary(std::sync::Arc::new(bytes));
        let resident = std::sync::Arc::make_mut(&mut self.database).push_face_info(face);
        self.resident.insert(id, resident);
        resident
    }
}

impl Default for Renderer {
    fn default() -> Self {
        let mut options = usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        // Pango/librsvg falls back to sans-serif for an explicit unknown family
        // (RtSvg commonly exports AAP_Font3 and AAR_Font1). usvg's selector always
        // falls back to serif. Preserve the implicit Times default while matching
        // the reference renderer for explicit missing families.
        let font_cache = std::sync::Arc::new(std::sync::Mutex::new(FontCache {
            database: options.fontdb.clone(),
            selected: Default::default(),
            resident: Default::default(),
        }));
        let selection_cache = font_cache.clone();
        options.font_resolver.select_font = Box::new(move |font, database| {
            let mut cache = selection_cache.lock().unwrap();
            if let Some(id) = cache.selected.get(font).copied() {
                *database = cache.database.clone();
                return id;
            }
            let mut families: Vec<_> = font
                .families()
                .iter()
                .map(|family| match family {
                    usvg::FontFamily::Named(name) => usvg::fontdb::Family::Name(name),
                    usvg::FontFamily::Serif => usvg::fontdb::Family::Serif,
                    usvg::FontFamily::SansSerif => usvg::fontdb::Family::SansSerif,
                    usvg::FontFamily::Cursive => usvg::fontdb::Family::Cursive,
                    usvg::FontFamily::Fantasy => usvg::fontdb::Family::Fantasy,
                    usvg::FontFamily::Monospace => usvg::fontdb::Family::Monospace,
                })
                .collect();
            let implicit_serif = font.families().iter().any(|family|matches!(family,usvg::FontFamily::Named(name) if name.eq_ignore_ascii_case("Times New Roman")));
            families.push(if implicit_serif {
                usvg::fontdb::Family::Serif
            } else {
                usvg::fontdb::Family::SansSerif
            });
            families.push(usvg::fontdb::Family::Serif);
            let id = cache.database.query(&usvg::fontdb::Query {
                families: &families,
                weight: usvg::fontdb::Weight(font.weight()),
                stretch: font.stretch().into(),
                style: font.style().into(),
            });
            let id = id.map(|id| cache.make_resident(id));
            cache.selected.insert(font.clone(), id);
            *database = cache.database.clone();
            id
        });
        let fallback = usvg::FontResolver::default_fallback_selector();
        options.font_resolver.select_fallback = Box::new(move |character, excluded, database| {
            let mut cache = font_cache.lock().unwrap();
            let id = fallback(character, excluded, &mut cache.database)
                .map(|id| cache.make_resident(id));
            *database = cache.database.clone();
            id
        });
        Self { options }
    }
}

impl Renderer {
    pub fn render(&self, content: &str, parsed: &Parsed) -> Result<(Vec<u8>, u32, u32)> {
        let content = normalize_paints(content)?;
        let content = inline_placeholders(&content)?;
        let tree = usvg::Tree::from_str(&content, &self.options)
            .context("Не удалось разобрать SVG для отрисовки")?;
        let size = tree.size().to_int_size();
        if u64::from(size.width()) * u64::from(size.height()) > 100_000_000 {
            bail!("Размер SVG превышает 100 миллионов пикселей");
        }
        let mut image = tiny_skia::Pixmap::new(size.width(), size.height())
            .context("Не удалось выделить память для SVG")?;
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut image.as_mut());
        let width = size.width() as f64;
        let height = size.height() as f64;
        let radius = (width / 160.0).round().clamp(10.0, 18.0);
        let step = radius * 2.0 + 6.0;
        let cols = ((width - 20.0) / step).floor().max(1.0) as usize;
        let mut fallback = 0;
        let mut overlay = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">"
        );
        for marker in &parsed.markers {
            let (x, y) = if let Some(point) = marker.point {
                (
                    point.x
                        * parsed
                            .view_width
                            .filter(|w| *w > 0.0)
                            .map(|w| width / w)
                            .unwrap_or(1.0),
                    point.y
                        * parsed
                            .view_height
                            .filter(|h| *h > 0.0)
                            .map(|h| height / h)
                            .unwrap_or(1.0),
                )
            } else {
                let position = (
                    10.0 + (fallback % cols) as f64 * step,
                    10.0 + (fallback / cols) as f64 * step,
                );
                fallback += 1;
                position
            };
            // Keep JavaScript's clamp behavior even for tiny source canvases.
            let x = x.min(width - radius - 2.0).max(radius + 2.0);
            let y = y.min(height - radius - 2.0).max(radius + 2.0);
            let text = marker.index.to_string();
            let font_size = (radius * 0.95).round().clamp(10.0, 15.0);
            let font_size = if text.len() >= 3 {
                (font_size - 3.0).max(9.0)
            } else {
                font_size
            };
            overlay.push_str(&format!("<g><circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{radius}\" fill=\"#d62828\" stroke=\"white\" stroke-width=\"1.5\"/><text x=\"{x:.2}\" y=\"{:.2}\" font-family=\"Arial, sans-serif\" font-size=\"{font_size}\" font-weight=\"700\" fill=\"#ffffff\" text-anchor=\"middle\">{text}</text></g>",y+font_size*0.35));
        }
        overlay.push_str("</svg>");
        let overlay = usvg::Tree::from_str(&overlay, &self.options)?;
        resvg::render(
            &overlay,
            tiny_skia::Transform::identity(),
            &mut image.as_mut(),
        );
        let png = image.encode_png()?;
        Ok((png, size.width(), size.height()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resident_fonts_preserve_data_ids_and_parallel_rendering() -> Result<()> {
        let renderer = Renderer::default();
        let database = renderer.options.fontdb.clone();
        if let Some(face) = database
            .faces()
            .find(|face| matches!(face.source, usvg::fontdb::Source::File(_)))
        {
            let original = face.id;
            let mut cache = FontCache {
                database: database.clone(),
                selected: Default::default(),
                resident: Default::default(),
            };
            let resident = cache.make_resident(original);
            assert_ne!(original, resident);
            assert_eq!(resident, cache.make_resident(original));
            assert_eq!(cache.database.faces().count(), database.faces().count() + 1);
            assert!(cache.database.face(original).is_some());
            assert_eq!(
                database.with_face_data(original, |bytes, index| (bytes.to_vec(), index)),
                cache
                    .database
                    .with_face_data(resident, |bytes, index| (bytes.to_vec(), index)),
            );
        }
        let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="180" height="70"><text x="5" y="25" font-family="sans-serif">Насос 42</text><text x="5" y="55" font-family="serif">Клапан 17</text></svg>"#;
        let expected = renderer.render(source, &Parsed::default())?.0;
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| renderer.render(source, &Parsed::default()).unwrap().0))
                .collect();
            for handle in handles {
                assert_eq!(handle.join().unwrap(), expected);
            }
        });
        Ok(())
    }

    #[test]
    fn unknown_fonts_use_sans_but_implicit_fonts_keep_serif() -> Result<()> {
        let renderer = Renderer::default();
        let draw = |family: &str| -> Result<Vec<u8>> {
            let content = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="150" height="60"><text x="5" y="30" font-size="20" {family}>Насос 42</text></svg>"#
            );
            Ok(renderer.render(&content, &Parsed::default())?.0)
        };
        assert_eq!(
            draw(r#"font-family="NppUnknownFontForRegression""#)?,
            draw(r#"font-family="sans-serif""#)?
        );
        assert_eq!(draw("")?, draw(r#"font-family="serif""#)?);
        Ok(())
    }
    #[test]
    fn symbolic_palette_names_inherit_instead_of_covering_text() -> Result<()> {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60" fill="none" stroke="black"><rect x="10" y="10" width="50" height="30" fill="Col_Grey2"/><g stroke="Col_Red"><line x1="70" y1="10" x2="70" y2="50"/></g></svg>"#;
        let normalized = normalize_paints(svg)?;
        assert!(!normalized.contains("Col_Grey2"));
        let tree = usvg::Tree::from_str(&normalized, &usvg::Options::default())?;
        let mut image = tiny_skia::Pixmap::new(100, 60).unwrap();
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut image.as_mut());
        assert_eq!(image.pixel(30, 30).unwrap().alpha(), 0);
        assert!(image.pixel(10, 20).unwrap().alpha() > 0);
        assert!(image.pixel(70, 30).unwrap().alpha() > 0);
        Ok(())
    }
    #[test]
    fn image_stylesheets_keep_the_original_element_type() -> Result<()> {
        let uri = format!("data:image/svg+xml;base64,{}", STANDARD.encode(PLACEHOLDER));
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60"><style>image {{ display:none }}</style><image x="10" y="10" width="50" height="30" href="{uri}"/></svg>"#
        );
        let prepared = inline_placeholders(&normalize_paints(&source)?)?;
        let tree = usvg::Tree::from_str(&prepared, &usvg::Options::default())?;
        let mut image = tiny_skia::Pixmap::new(100, 60).unwrap();
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut image.as_mut());
        assert_eq!(image.pixel(30, 30).unwrap().alpha(), 0);
        Ok(())
    }
    #[test]
    fn placeholder_viewport_preserves_aspect_ratio_and_transform() -> Result<()> {
        let uri = format!("data:image/svg+xml;base64,{}", STANDARD.encode(PLACEHOLDER));
        for body in [
            format!(r#"<image x="10" y="5" width="50" height="30" href="{uri}"/>"#),
            format!(
                r#"<image x="10" y="5" width="50" height="30" transform="translate(10 5)" href="{uri}"><title>K</title></image>"#
            ),
            format!(
                r#"<image x="10" y="5" width="50" height="30" preserveAspectRatio="none" href="{uri}"/>"#
            ),
        ] {
            let source = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60">{body}</svg>"#
            );
            let inline = inline_placeholders(&source)?;
            let options = usvg::Options::default();
            let raster = |text: &str| -> Result<_> {
                let tree = usvg::Tree::from_str(text, &options)?;
                let mut image = tiny_skia::Pixmap::new(100, 60).unwrap();
                resvg::render(&tree, tiny_skia::Transform::identity(), &mut image.as_mut());
                Ok(image)
            };
            let a = raster(&source)?;
            let b = raster(&inline)?;
            let mean_error = a
                .data()
                .iter()
                .zip(b.data())
                .map(|(a, b)| a.abs_diff(*b) as f64)
                .sum::<f64>()
                / a.data().len() as f64;
            assert!(
                mean_error < 2.0,
                "Viewport changed symbol layout: {mean_error}"
            );
        }
        Ok(())
    }
    #[test]
    fn nested_marker_order_and_identity_priorities() -> Result<()> {
        let parsed = parse(
            r#"<svg viewBox="0 0 200 100"><g><title>Group</title><rect x="2" y="3" width="4" height="5"><title>A &amp; B</title><rt:dyn type="PointID" value="P"/><rt:dyn type="KKS" value="K"/><rt:dyn type="FKKS" value="F"/></rect></g><text><title>''</title></text></svg>"#,
        )?;
        assert_eq!(
            (parsed.view_width, parsed.view_height),
            (Some(200.0), Some(100.0))
        );
        assert_eq!(
            parsed.markers.iter().map(|m| m.index).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(parsed.markers[0].title, "A & B");
        assert_eq!(parsed.markers[0].kks.as_deref(), Some("F"));
        assert!(parsed.markers[0].is_mismatch);
        assert_eq!(parsed.markers[1].point, Some(Point { x: 6.0, y: 3.0 }));
        assert_eq!(parsed.markers[2].title, "(empty title)");
        assert!(!parsed.markers[2].is_mismatch);
        Ok(())
    }
    const FRAME: &str = "<svg width=\"100\" height=\"100\"><title>Насос</title><image x=\"10\" y=\"20\" width=\"30\" height=\"40\" xlink:href=\"motor.svg\"><title>41ABC</title><rt:dyn type=\"KKS\" value=\"41ABC\"/></image></svg>";
    #[test]
    fn passport_regression() -> Result<()> {
        let (bytes, _, _) = WINDOWS_1251.encode(FRAME);
        let (decoded, encoding) = decode(&bytes);
        assert_eq!(encoding, "windows-1251");
        assert!(decoded.contains("Насос"));
        let parsed = parse(FRAME)?;
        assert_eq!(parsed.markers.len(), 1);
        assert_eq!(parsed.markers[0].point, Some(Point { x: 40.0, y: 20.0 }));
        assert_eq!(parsed.markers[0].kks.as_deref(), Some("41ABC"));
        assert_eq!(
            parse(&FRAME.replace("<title>41ABC</title>", ""))?.markers[0]
                .kks
                .as_deref(),
            Some("41ABC")
        );
        assert!(!utf8_xml("<svg>&#4;&#x4;</svg>").contains("&#"));
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("frame.svg");
        assert_eq!(
            prepare_resources(&utf8_xml(FRAME), &path).1,
            vec!["motor.svg"]
        );
        fs::write(
            tmp.path().join("motor.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"30\" height=\"40\"/></svg>",
        )?;
        assert!(prepare_resources(&utf8_xml(FRAME), &path).1.is_empty());
        assert_eq!(
            prepare_resources(
                &utf8_xml(&FRAME.replace("motor.svg", "https://example.com/m.svg")),
                &path
            )
            .1,
            vec!["https://example.com/m.svg"]
        );
        Ok(())
    }
}
