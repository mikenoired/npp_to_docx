fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "Usage: render_svg INPUT.svg OUTPUT.png");
    let content = npp_core::svg::decode(&std::fs::read(&args[1])?).0;
    let renderer = npp_core::svg::Renderer::default();
    let (png, _, _) = renderer.render(&content, &npp_core::svg::Parsed::default())?;
    std::fs::write(&args[2], png)?;
    Ok(())
}
