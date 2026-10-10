use crate::svg::Marker;
use anyhow::Result;
use std::{
    io::{Cursor, Write},
    path::Path,
};
use zip::{ZipWriter, write::SimpleFileOptions};

pub fn escape(text: &str) -> String {
    quick_xml::escape::escape(text).into_owned()
}

fn paragraph(text: &str, properties: &str, run_properties: &str) -> String {
    format!(
        "<w:p>{properties}<w:r>{run_properties}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        escape(text)
    )
}

fn cell(text: &str, width: u32, center: bool, bold: bool, red: bool) -> String {
    let props = if center {
        "<w:pPr><w:jc w:val=\"center\"/></w:pPr>"
    } else {
        ""
    };
    let mut run = String::new();
    if bold {
        run.push_str("<w:b/><w:bCs/>");
    }
    if red {
        run.push_str("<w:color w:val=\"CC0000\"/>");
    }
    let run = if run.is_empty() {
        run
    } else {
        format!("<w:rPr>{run}</w:rPr>")
    };
    format!(
        "<w:tc><w:tcPr><w:tcW w:w=\"{width}\" w:type=\"dxa\"/></w:tcPr>{}</w:tc>",
        paragraph(text, props, &run)
    )
}

pub fn build(
    png: &[u8],
    width: u32,
    height: u32,
    markers: &[Marker],
    frame: &str,
    missing: &[String],
) -> Result<Vec<u8>> {
    let image_width = (width as f64)
        .min(960.0)
        .min(400.0 * width as f64 / height as f64);
    let image_height = (height as f64 / width as f64 * image_width)
        .round()
        .max(1.0);
    let cx = (image_width * 9525.0).round() as u64;
    let cy = (image_height * 9525.0).round() as u64;
    let mut body = paragraph(
        &format!("Паспорт кадра {frame}"),
        "<w:pPr><w:pStyle w:val=\"Title\"/></w:pPr>",
        "",
    );
    body.push_str(&paragraph(
        &format!(
            "Элементов: {}. Номера на схеме соответствуют строкам таблицы.",
            markers.len()
        ),
        "",
        "",
    ));
    if !missing.is_empty() {
        body.push_str(&paragraph(&format!("Отсутствуют изображения подмоделей: {}. На схеме они обозначены прямоугольниками с крестом.",missing.join(", ")),"",""));
    }
    body.push_str(&format!(r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="1" name="Схема кадра"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="frame.png"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="image"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p><w:p/>"#));
    if markers.is_empty() {
        body.push_str(&paragraph(
            "Элементов с title, KKS или PointID не найдено",
            "",
            "",
        ));
    } else {
        body.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblBorders>");
        for edge in ["top", "left", "bottom", "right", "insideH", "insideV"] {
            body.push_str(&format!(
                "<w:{edge} w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>"
            ));
        }
        body.push_str("</w:tblBorders><w:tblLayout w:type=\"fixed\"/></w:tblPr><w:tblGrid>");
        let widths = [700, 2500, 4200, 1600];
        for width in widths {
            body.push_str(&format!("<w:gridCol w:w=\"{width}\"/>"));
        }
        body.push_str("</w:tblGrid><w:tr><w:trPr><w:tblHeader/></w:trPr>");
        for (i, text) in ["№", "KKS", "Текстовое описание", "Подмодель"]
            .iter()
            .enumerate()
        {
            body.push_str(&cell(text, widths[i], i == 0, true, false));
        }
        body.push_str("</w:tr>");
        for marker in markers {
            body.push_str("<w:tr>");
            for (i, text) in [
                marker.index.to_string(),
                marker.kks.clone().unwrap_or(marker.title.clone()),
                marker.description.clone().unwrap_or_default(),
                marker.submodel.clone().unwrap_or_default(),
            ]
            .iter()
            .enumerate()
            {
                body.push_str(&cell(
                    text,
                    widths[i],
                    i == 0,
                    false,
                    i == 0 && marker.is_mismatch,
                ));
            }
            body.push_str("</w:tr>");
        }
        body.push_str("</w:tbl>");
        let mismatches: Vec<_> = markers.iter().filter(|m| m.is_mismatch).collect();
        if !mismatches.is_empty() {
            body.push_str("<w:p/>");
            body.push_str(&paragraph(
                "Несовпадения title/KKS:",
                "",
                "<w:rPr><w:b/><w:bCs/></w:rPr>",
            ));
            for marker in mismatches {
                body.push_str(&paragraph(
                    &format!(
                        "№ {}: title=\"{}\", KKS=\"{}\"",
                        marker.index,
                        marker.title,
                        marker.kks.as_deref().unwrap_or("")
                    ),
                    "",
                    "",
                ));
            }
        }
    }
    body.push_str("<w:sectPr><w:pgSz w:w=\"16838\" w:h=\"11906\" w:orient=\"landscape\"/><w:pgMar w:top=\"720\" w:right=\"720\" w:bottom=\"720\" w:left=\"720\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/><w:pgNumType/><w:docGrid w:linePitch=\"360\"/></w:sectPr>");
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}</w:body></w:document>"#
    );
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name,data) in [
        ("[Content_Types].xml",r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#.as_bytes()),
        ("_rels/.rels",r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="document" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.as_bytes()),
        ("word/document.xml",document.as_bytes()),
        ("word/styles.xml",include_bytes!("../assets/styles.xml")),
        ("word/_rels/document.xml.rels",r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="styles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="image" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/frame.png"/></Relationships>"#.as_bytes()),
        ("word/media/frame.png",png),
    ] {
        zip.start_file(name,options)?;
        zip.write_all(data)?;
    }
    Ok(zip.finish()?.into_inner())
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes)?;
    Ok(())
}
