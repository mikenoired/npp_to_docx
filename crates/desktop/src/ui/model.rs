use super::*;
use std::{collections::BTreeSet, path::Path};

pub(super) fn config_path(name: &str) -> PathBuf {
    ProjectDirs::from("local", "npp", "NppToDocx")
        .map(|p| p.config_dir().join(name))
        .unwrap_or_else(|| PathBuf::from(name))
}
pub(super) fn workspace() -> PathBuf {
    UserDirs::new()
        .and_then(|p| p.document_dir().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("npp_to_docx")
}
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Settings {
    pub input_dir: String,
    pub svg_dir: String,
    pub output_dir: String,
    pub concurrency: usize,
    pub filter: String,
    pub limit: String,
    pub dark: bool,
}
impl Settings {
    pub fn load() -> Self {
        let mut s: Self = fs::read(config_path("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if s.input_dir.is_empty() {
            s.input_dir = workspace().join("input").to_string_lossy().into();
        }
        if s.output_dir.is_empty() {
            s.output_dir = workspace().join("output").to_string_lossy().into();
        }
        if !(1..=256).contains(&s.concurrency) {
            s.concurrency = batch::default_concurrency();
        }
        s
    }
    pub fn save(&self) -> Result<()> {
        save_json("settings.json", self)
    }
}
fn save_json<T: Serialize + ?Sized>(name: &str, value: &T) -> Result<()> {
    let path = config_path(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
pub(super) struct Job {
    pub id: String,
    pub label: String,
    pub options: Options,
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub warnings: usize,
    pub result: Option<BatchResult>,
    pub error: Option<String>,
    pub logs: VecDeque<String>,
    pub log_path: PathBuf,
    pub elapsed_ms: u64,
    #[serde(skip)]
    pub started: Option<Instant>,
}
impl Job {
    pub fn elapsed(&self) -> Duration {
        self.started
            .map(|s| s.elapsed())
            .unwrap_or_else(|| Duration::from_millis(self.elapsed_ms))
    }
    pub fn load() -> Vec<Self> {
        let mut jobs: Vec<Self> = fs::read(config_path("history.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        for job in &mut jobs {
            if job.status == "Выполняется" {
                job.status = "Прервано".into();
                job.error = Some(
                    "Приложение закрыли до завершения пакета. Запустите обработку заново.".into(),
                );
            }
        }
        jobs
    }
    pub fn save(jobs: &[Self]) -> Result<()> {
        save_json("history.json", &jobs[jobs.len().saturating_sub(30)..])
    }
}
pub(super) struct Catalog {
    pub directory: PathBuf,
    pub index: SearchIndex,
    pub submodels: Vec<(String, usize)>,
    pub frame_count: usize,
}
impl Catalog {
    fn load(directory: &Path) -> Option<Self> {
        let index = SearchIndex::read(directory).ok()?;
        let submodels = index.submodels("");
        let frame_count = index
            .records
            .iter()
            .map(|r| &r.frame_name)
            .collect::<BTreeSet<_>>()
            .len();
        Some(Self {
            directory: directory.into(),
            index,
            submodels,
            frame_count,
        })
    }
}
pub(super) struct Database {
    pub name: &'static str,
    pub format: Option<&'static str>,
}
pub(super) struct Manifest {
    pub databases: Vec<Database>,
    pub svg_dir: PathBuf,
    pub frames: Vec<String>,
    pub matched: usize,
    pub selected: usize,
    pub output_exists: bool,
    pub error: Option<String>,
}
impl Manifest {
    pub fn inspect(options: &Options) -> (Self, Option<Catalog>) {
        let input = &options.input_dir;
        let candidate = input.join("svg");
        let svg_dir = options.svg_dir.clone().unwrap_or_else(|| {
            if candidate.is_dir() {
                candidate
            } else {
                input.clone()
            }
        });
        let databases: Vec<_> = npp_core::db::DATABASES
            .iter()
            .map(|name| Database {
                name,
                format: if input.join(format!("{name}.csv")).is_file() {
                    Some("CSV")
                } else if input.join(format!("{name}.dmp")).is_file() {
                    Some("DMP")
                } else {
                    None
                },
            })
            .collect();
        let mut error = None;
        let mut frames = Vec::new();
        match fs::read_dir(&svg_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry)
                            if entry.file_type().is_ok_and(|t| t.is_file())
                                && entry
                                    .path()
                                    .extension()
                                    .is_some_and(|e| e.eq_ignore_ascii_case("svg")) =>
                        {
                            frames.push(entry.file_name().to_string_lossy().into_owned())
                        }
                        Err(e) => error = Some(format!("Не удалось прочитать папку SVG: {e}")),
                        _ => {}
                    }
                }
            }
            Err(e) => error = Some(format!("Папка SVG недоступна: {}. {e}", svg_dir.display())),
        }
        if !input.is_dir() {
            error = Some("Выберите существующую папку с базами описаний.".into());
        }
        if options.output_dir.as_os_str().is_empty() || options.output_dir.is_file() {
            error = Some("Укажите папку для документов, а не отдельный файл.".into());
        }
        frames.sort();
        let needle = options.r#match.as_deref().unwrap_or("").to_lowercase();
        let matching: Vec<_> = frames
            .into_iter()
            .filter(|n: &String| n.to_lowercase().contains(&needle))
            .collect();
        let matched = matching.len();
        let selected = options.limit.unwrap_or(matched).min(matched);
        let catalog = Catalog::load(&options.output_dir);
        (
            Self {
                databases,
                svg_dir,
                frames: matching,
                matched,
                selected,
                output_exists: options.output_dir.is_dir(),
                error,
            },
            catalog,
        )
    }
    pub fn ready(&self) -> bool {
        self.error.is_none()
            && self.selected > 0
            && self.databases.iter().all(|db| db.format.is_some())
    }
}
pub(super) fn duration(value: Duration) -> String {
    let s = value.as_secs();
    if s >= 3600 {
        format!("{} ч {:02} мин", s / 3600, s / 60 % 60)
    } else if s >= 60 {
        format!("{} мин {:02} с", s / 60, s % 60)
    } else {
        format!("{s} с")
    }
}
pub(super) fn count(value: usize) -> String {
    let digits = value.to_string();
    let mut output = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            output.push(' ');
        }
        output.push(ch);
    }
    output
}
