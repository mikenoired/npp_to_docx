mod assets;
mod theme;
mod ui;

fn main() -> anyhow::Result<()> {
    if std::env::args_os().len() > 1 {
        npp_core::cli::run()
    } else {
        ui::run()
    }
}
