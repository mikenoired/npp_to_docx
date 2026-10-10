use crate::batch::{self, Options, Progress, SearchIndex};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf, time::Instant};

#[derive(Parser)]
#[command(version, about = "Паспорта SVG-кадров в DOCX")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Преобразовать SVG и построить индекс поиска.
    Convert {
        #[arg(long, default_value = "input")]
        input: PathBuf,
        #[arg(long)]
        svg: Option<PathBuf>,
        #[arg(long, default_value = "output")]
        output: PathBuf,
        #[arg(long,default_value_t=batch::default_concurrency())]
        concurrency: usize,
        #[arg(long)]
        r#match: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        result_json: Option<PathBuf>,
    },
    /// Подготовить CSV из DMP базы.
    Prepare {
        #[arg(long, default_value = "input")]
        input: PathBuf,
    },
    /// Найти видеокадры с заданной подмоделью.
    Search {
        #[arg(long, default_value = "output")]
        output: PathBuf,
        #[arg(long)]
        submodel: String,
    },
    /// Проверить маркеры и координаты без генерации документов.
    Inspect {
        #[arg(long)]
        svg: PathBuf,
    },
}

pub fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Convert {
            input,
            svg,
            output,
            concurrency,
            r#match,
            limit,
            result_json,
        } => {
            let start = Instant::now();
            let result = batch::process(
                &Options {
                    input_dir: input,
                    svg_dir: svg,
                    output_dir: output,
                    concurrency,
                    r#match,
                    limit,
                },
                &|event| {
                    if let Progress::Log(line) = event {
                        eprintln!("{line}");
                    }
                },
            )?;
            let mut json = serde_json::to_value(&result)?;
            json["elapsed_ms"] = serde_json::json!(start.elapsed().as_secs_f64() * 1000.0);
            let json = serde_json::to_string_pretty(&json)?;
            if let Some(path) = result_json {
                fs::write(path, &json)?;
            }
            println!("{json}");
            if result.failed > 0 {
                anyhow::bail!("Не удалось преобразовать {} кадров", result.failed);
            }
        }
        Command::Prepare { input } => {
            for file in crate::db::prepare(&input)? {
                println!("{}", file.display());
            }
        }
        Command::Search { output, submodel } => println!(
            "{}",
            serde_json::to_string_pretty(&SearchIndex::read(&output)?.search(&submodel))?
        ),
        Command::Inspect { svg } => println!(
            "{}",
            serde_json::to_string_pretty(&crate::svg::parse(
                &crate::svg::decode(&fs::read(svg)?).0
            )?)?
        ),
    }
    Ok(())
}
