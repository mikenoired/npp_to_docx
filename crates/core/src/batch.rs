use crate::{
    db::DescriptionIndex,
    docx,
    svg::{self, Renderer},
};
use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

fn filename_compare(left: &str, right: &str) -> std::cmp::Ordering {
    static COLLATOR: std::sync::LazyLock<icu_collator::CollatorBorrowed<'static>> =
        std::sync::LazyLock::new(|| {
            icu_collator::Collator::try_new(Default::default(), Default::default())
                .expect("Compiled ICU collation data")
        });
    COLLATOR.compare(left, right)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub input_dir: PathBuf,
    pub svg_dir: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub concurrency: usize,
    pub r#match: Option<String>,
    pub limit: Option<usize>,
}

pub fn default_concurrency() -> usize {
    (std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        / 2)
    .clamp(1, 6)
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub success: usize,
    pub failed: usize,
    pub total_markers: usize,
    pub total_mismatches: usize,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub enum Progress {
    Log(String),
    Counts {
        completed: usize,
        total: usize,
        success: usize,
        failed: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecord {
    pub frame_name: String,
    pub marker_index: usize,
    pub submodel: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndex {
    pub created_at: String,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub records: Vec<SearchRecord>,
}

impl SearchIndex {
    pub fn read(output: &Path) -> Result<Self> {
        let path = output.join("search-index.json");
        let bytes = fs::read(&path).with_context(|| {
            format!(
                "Индекс поиска не найден: {}. Запустите обработку.",
                path.display()
            )
        })?;
        serde_json::from_slice(&bytes)
            .context("Поисковый индекс поврежден. Запустите обработку заново.")
    }
    pub fn search(&self, query: &str) -> Vec<SearchRecord> {
        let needle = query.trim().to_lowercase();
        self.records
            .iter()
            .filter(|record| record.submodel.trim().to_lowercase() == needle)
            .cloned()
            .collect()
    }
    pub fn submodels(&self, query: &str) -> Vec<(String, usize)> {
        let needle = query.trim().to_lowercase();
        let mut counts = BTreeMap::new();
        for record in &self.records {
            if record.submodel.to_lowercase().contains(&needle) {
                *counts.entry(record.submodel.clone()).or_insert(0) += 1;
            }
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(filename_compare(&a.0, &b.0)));
        counts
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameReport {
    frame: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    markers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_resources: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn process(options: &Options, progress: &(impl Fn(Progress) + Sync)) -> Result<BatchResult> {
    ensure!(
        (1..=256).contains(&options.concurrency),
        "Число параллельных обработок должно быть от 1 до 256"
    );
    ensure!(options.limit != Some(0), "Лимит должен быть положительным");
    let input = fs::canonicalize(&options.input_dir).context("Входная папка не найдена")?;
    let candidate = input.join("svg");
    let svg_dir = options.svg_dir.clone().unwrap_or_else(|| {
        if candidate.is_dir() {
            candidate
        } else {
            input.clone()
        }
    });
    fs::create_dir_all(&options.output_dir)?;
    let output = fs::canonicalize(&options.output_dir)?;
    let descriptions = DescriptionIndex::load(&input)?;
    let mut files: Vec<_> = fs::read_dir(&svg_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")))
        .collect();
    files.sort_by(|a, b| {
        filename_compare(
            &a.file_name().unwrap().to_string_lossy(),
            &b.file_name().unwrap().to_string_lossy(),
        )
    });
    if let Some(needle) = &options.r#match {
        let needle = needle.to_lowercase();
        files.retain(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .to_lowercase()
                .contains(&needle)
        });
    }
    if let Some(limit) = options.limit {
        files.truncate(limit);
    }
    if files.is_empty() {
        return Ok(BatchResult {
            summary: "SVG-файлы не найдены.".into(),
            ..Default::default()
        });
    }
    let total = files.len();
    progress(Progress::Log(format!(
        "Starting conversion. Files: {total}"
    )));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.concurrency.min(total))
        .build()?;
    let done = AtomicUsize::new(0);
    let success = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let reports = Mutex::new(Vec::new());
    let records = Mutex::new(Vec::new());
    let renderer = Renderer::default();
    let totals: Vec<(usize,usize)> = pool.install(||files.par_iter().map(|path| {
        let frame = path.file_name().unwrap().to_string_lossy().into_owned();
        let start = Instant::now();
        let conversion = (|| -> Result<_> {
            let raw = fs::read(path)?;
            let (content,encoding) = svg::decode(&raw);
            let mut parsed = svg::parse(&content)?;
            for marker in &mut parsed.markers {marker.description = descriptions.lookup(marker.kks.as_deref());}
            let (prepared,missing) = svg::prepare_resources(&svg::utf8_xml(&content),path);
            let (png,width,height) = renderer.render(&prepared,&parsed)?;
            let docx = docx::build(&png,width,height,&parsed.markers,&frame,&missing)?;
            let mut destination = output.join(path.file_name().unwrap());destination.set_extension("docx");
            docx::write(&destination,&docx)?;
            Ok((parsed,missing,encoding))
        })();
        let counts = match conversion {
            Ok((parsed,missing,encoding)) => {
                success.fetch_add(1,Ordering::SeqCst);
                let count = parsed.markers.len();
                let mismatches = parsed.markers.iter().filter(|m|m.is_mismatch).count();
                let new_records = parsed.markers.into_iter().filter(|m|m.submodel.as_ref().is_some_and(|s|!s.trim().is_empty())).map(|m|SearchRecord{frame_name:frame.clone(),marker_index:m.index,submodel:m.submodel.unwrap(),title:m.title,kks:m.kks,description:m.description});
                records.lock().unwrap().extend(new_records);
                if !missing.is_empty() {progress(Progress::Log(format!("WARNING {frame}: missing submodels: {}",missing.join(", "))));}
                reports.lock().unwrap().push(FrameReport{frame:frame.clone(),markers:Some(count),missing_resources:Some(missing),error:None});
                progress(Progress::Log(format!("OK {frame} | markers={count} | mismatches={mismatches} | encoding={encoding} | {}ms",start.elapsed().as_millis())));
                (count,mismatches)
            },
            Err(error) => {
                failed.fetch_add(1,Ordering::SeqCst);
                let message = format!("{error:#}");
                reports.lock().unwrap().push(FrameReport{frame:frame.clone(),markers:None,missing_resources:None,error:Some(message.clone())});
                progress(Progress::Log(format!("FAIL {frame}: {message}")));
                (0,0)
            }
        };
        let completed = done.fetch_add(1,Ordering::SeqCst)+1;
        progress(Progress::Counts{completed,total,success:success.load(Ordering::SeqCst),failed:failed.load(Ordering::SeqCst)});
        counts
    }).collect());
    let mut reports = reports.into_inner().unwrap();
    reports.sort_by(|a, b| filename_compare(&a.frame, &b.frame));
    fs::write(
        output.join("passport-report.json"),
        serde_json::to_vec_pretty(&reports)?,
    )?;
    let mut records = records.into_inner().unwrap();
    records.sort_by(|a, b| {
        filename_compare(&a.frame_name, &b.frame_name).then(a.marker_index.cmp(&b.marker_index))
    });
    let index = SearchIndex {
        created_at: chrono::Utc::now().to_rfc3339(),
        input_dir: input,
        output_dir: output.clone(),
        records,
    };
    let temp = output.join("search-index.json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(&index)?)?;
    fs::rename(temp, output.join("search-index.json"))?;
    let mut result = BatchResult {
        success: success.load(Ordering::SeqCst),
        failed: failed.load(Ordering::SeqCst),
        total_markers: totals.iter().map(|v| v.0).sum(),
        total_mismatches: totals.iter().map(|v| v.1).sum(),
        summary: String::new(),
    };
    result.summary = format!(
        "Готово. успешно={}, провально={}, всего_маркеров={}, всего_несовпадений={}",
        result.success, result.failed, result.total_markers, result.total_mismatches
    );
    progress(Progress::Log(result.summary.clone()));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filename_order_matches_electron_locale_compare() {
        let mut names = [
            "4SPPB0_1.svg",
            "4SPPB.svg",
            "4SPPB_AZ_1.svg",
            "0TLR.svg",
            "0_PCP.svg",
        ];
        names.sort_by(|a, b| filename_compare(a, b));
        assert_eq!(
            names,
            [
                "0_PCP.svg",
                "0TLR.svg",
                "4SPPB_AZ_1.svg",
                "4SPPB.svg",
                "4SPPB0_1.svg"
            ]
        );
    }
    #[test]
    fn errors_are_isolated_and_index_is_searchable() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        for name in crate::db::DATABASES {
            fs::write(
                tmp.path().join(format!("{name}.csv")),
                "PVID,PVDESCRIPTION,PVTEXT,PLC_ITEMID\nK,Описание,,K\n",
            )?;
        }
        fs::write(
            tmp.path().join("good.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><image href="motor.svg" x="5" y="5" width="10" height="10"><title>K</title></image></svg>"#,
        )?;
        fs::write(tmp.path().join("bad.svg"), "<svg>")?;
        let output = tmp.path().join("out");
        let options = Options {
            input_dir: tmp.path().into(),
            svg_dir: None,
            output_dir: output.clone(),
            concurrency: 2,
            r#match: None,
            limit: None,
        };
        let result = process(&options, &|_| {})?;
        assert_eq!((result.success, result.failed), (1, 1));
        let index = SearchIndex::read(&output)?;
        assert_eq!(
            index.search(" MOTOR.SVG ")[0].description.as_deref(),
            Some("Описание")
        );
        assert_eq!(index.submodels("mot"), vec![("motor.svg".into(), 1)]);
        // Replacing an existing index must also work, including on Windows.
        process(&options, &|_| {})?;
        fs::write(output.join("search-index.json"), "{")?;
        assert!(
            SearchIndex::read(&output)
                .unwrap_err()
                .to_string()
                .contains("поврежден")
        );
        Ok(())
    }
}
