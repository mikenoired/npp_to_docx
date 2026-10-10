use npp_core::svg;
fn main() -> anyhow::Result<()> {
    let path = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let start = std::time::Instant::now();
    let renderer = svg::Renderer::default();
    println!("fonts {:?}", start.elapsed());
    let content = svg::decode(&std::fs::read(&path)?).0;
    let parsed = svg::parse(&content)?;
    let (prepared, missing) = svg::prepare_resources(&svg::utf8_xml(&content), &path);
    println!("prepare {:?}", start.elapsed());
    let (png, w, h) = renderer.render(&prepared, &parsed)?;
    println!("render {:?}", start.elapsed());
    let docx = npp_core::docx::build(
        &png,
        w,
        h,
        &parsed.markers,
        path.file_name().unwrap().to_str().unwrap(),
        &missing,
    )?;
    println!("docx {:?} {} bytes", start.elapsed(), docx.len());
    Ok(())
}
