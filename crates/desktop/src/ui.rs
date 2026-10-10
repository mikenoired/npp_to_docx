use crate::theme::{self, Palette};
use anyhow::{Context as _, Result};
use directories::{ProjectDirs, UserDirs};
use gpui::{Context, prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Root, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use npp_core::batch::{self, BatchResult, Options, Progress, SearchIndex, SearchRecord};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod model;
mod views;
use model::{Catalog, Job, Manifest, Settings, count, duration, workspace};
actions!(desktop, [CreatePassports, FindFrame, ShowRuns]);

enum WorkerEvent {
    Progress(Progress),
    Done(std::result::Result<BatchResult, String>),
    Prepared(std::result::Result<(), String>),
}
struct Desktop {
    input: Entity<InputState>,
    svg: Entity<InputState>,
    output: Entity<InputState>,
    concurrency: Entity<InputState>,
    filter: Entity<InputState>,
    limit: Entity<InputState>,
    query: Entity<InputState>,
    settings: Settings,
    focus: FocusHandle,
    tab: usize,
    busy: bool,
    preparing: bool,
    advanced: bool,
    show_logs: bool,
    message: String,
    message_error: bool,
    jobs: Vec<Job>,
    running: Option<usize>,
    selected_job: Option<usize>,
    receiver: Option<mpsc::Receiver<WorkerEvent>>,
    inspection: Option<mpsc::Receiver<(u64, Manifest, Option<Catalog>)>>,
    inspection_due: Option<Instant>,
    revision: u64,
    manifest: Option<Manifest>,
    catalog: Option<Catalog>,
    results: Vec<SearchRecord>,
    groups: Vec<std::ops::Range<usize>>,
    expanded_frame: Option<usize>,
    searched: bool,
    page: usize,
    last_tick: Instant,
    last_history_save: Instant,
}
fn field(
    value: String,
    placeholder: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    cx.new(|cx| {
        let mut input = InputState::new(window, cx).placeholder(placeholder.to_string());
        input.set_value(value, window, cx);
        input
    })
}
impl Desktop {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        theme::apply(settings.dark, window, cx);
        let jobs = Job::load();
        let mut app = Self {
            input: field(
                settings.input_dir.clone(),
                "Папка с PLS_ANA_CONF и PLS_BIN_CONF",
                window,
                cx,
            ),
            svg: field(
                settings.svg_dir.clone(),
                "Автоматически: из входной папки",
                window,
                cx,
            ),
            output: field(settings.output_dir.clone(), "Папка для DOCX", window, cx),
            concurrency: field(settings.concurrency.to_string(), "1–256", window, cx),
            filter: field(settings.filter.clone(), "Все SVG", window, cx),
            limit: field(settings.limit.clone(), "Без лимита", window, cx),
            query: field(
                String::new(),
                "Имя подмодели, например DS_ana.svg",
                window,
                cx,
            ),
            advanced: !settings.filter.is_empty() || !settings.limit.is_empty(),
            settings,
            focus: cx.focus_handle(),
            tab: 0,
            busy: false,
            preparing: false,
            show_logs: false,
            message: String::new(),
            message_error: false,
            selected_job: jobs.len().checked_sub(1),
            jobs,
            running: None,
            receiver: None,
            inspection: None,
            inspection_due: None,
            revision: 0,
            manifest: None,
            catalog: None,
            results: Vec::new(),
            groups: Vec::new(),
            expanded_frame: None,
            searched: false,
            page: 0,
            last_tick: Instant::now(),
            last_history_save: Instant::now(),
        };
        for (i, entity) in [
            app.input.clone(),
            app.svg.clone(),
            app.output.clone(),
            app.concurrency.clone(),
            app.filter.clone(),
            app.limit.clone(),
        ]
        .into_iter()
        .enumerate()
        {
            cx.subscribe(&entity, move |app, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    if i == 2 {
                        app.catalog = None;
                        app.results.clear();
                        app.groups.clear();
                        app.expanded_frame = None;
                        app.searched = false;
                    }
                    app.schedule_inspection();
                    cx.notify();
                }
            })
            .detach();
        }
        cx.subscribe_in(
            &app.query,
            window,
            |app, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    app.searched = false;
                    app.results.clear();
                    app.groups.clear();
                    app.expanded_frame = None;
                    app.page = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => app.search(window, cx),
                _ => {}
            },
        )
        .detach();
        app.schedule_inspection();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |app, cx| app.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        app
    }
    fn value(input: &Entity<InputState>, cx: &App) -> String {
        input.read(cx).value().to_string()
    }
    fn persist(&mut self, cx: &App) -> Result<()> {
        self.settings.input_dir = Self::value(&self.input, cx);
        self.settings.svg_dir = Self::value(&self.svg, cx);
        self.settings.output_dir = Self::value(&self.output, cx);
        self.settings.filter = Self::value(&self.filter, cx);
        self.settings.limit = Self::value(&self.limit, cx);
        if let Ok(n) = Self::value(&self.concurrency, cx).trim().parse::<usize>()
            && (1..=256).contains(&n)
        {
            self.settings.concurrency = n;
        }
        self.settings.save()
    }
    fn options(&self, cx: &App) -> Result<Options> {
        let svg = Self::value(&self.svg, cx);
        let filter = Self::value(&self.filter, cx);
        let limit = Self::value(&self.limit, cx);
        let concurrency = Self::value(&self.concurrency, cx)
            .trim()
            .parse::<usize>()
            .context("Введите число одновременных обработок от 1 до 256.")?;
        anyhow::ensure!(
            (1..=256).contains(&concurrency),
            "Число обработок должно быть от 1 до 256."
        );
        let limit = if limit.trim().is_empty() {
            None
        } else {
            let n = limit
                .trim()
                .parse::<usize>()
                .context("Лимит должен быть положительным числом.")?;
            anyhow::ensure!(n > 0, "Лимит должен быть больше нуля.");
            Some(n)
        };
        Ok(Options {
            input_dir: Self::value(&self.input, cx).trim().into(),
            svg_dir: (!svg.trim().is_empty()).then(|| svg.trim().into()),
            output_dir: Self::value(&self.output, cx).trim().into(),
            concurrency,
            r#match: (!filter.is_empty()).then_some(filter),
            limit,
        })
    }
    fn schedule_inspection(&mut self) {
        self.revision += 1;
        self.manifest = None;
        self.inspection_due = Some(Instant::now() + Duration::from_millis(250));
    }
    fn inspect(&mut self, cx: &App) {
        self.inspection_due = None;
        if let Err(e) = self.persist(cx) {
            self.message = format!("Не удалось сохранить настройки: {e}");
            self.message_error = true;
        }
        if let Ok(options) = self.options(cx) {
            let revision = self.revision;
            let (sender, receiver) = mpsc::channel();
            self.inspection = Some(receiver);
            std::thread::spawn(move || {
                let (manifest, catalog) = Manifest::inspect(&options);
                let _ = sender.send((revision, manifest, catalog));
            });
        }
    }
    fn error(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.message = message.into();
        self.message_error = true;
        cx.notify();
    }
    fn navigate(&mut self, tab: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = tab;
        self.message.clear();
        if tab == 1 {
            self.query.read(cx).focus_handle(cx).focus(window);
        } else {
            self.focus.focus(window);
        }
        cx.notify();
    }
    fn start(&mut self, prepare: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let options = match self.options(cx).and_then(|o| {
            self.persist(cx)?;
            Ok(o)
        }) {
            Ok(o) => o,
            Err(e) => {
                self.error(e.to_string(), cx);
                return;
            }
        };
        if !prepare && !self.manifest.as_ref().is_some_and(Manifest::ready) {
            self.error(
                "Проверьте исходные данные: нужны SVG и обе базы описаний.",
                cx,
            );
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.busy = true;
        self.preparing = prepare;
        self.message.clear();
        if prepare {
            std::thread::spawn(move || {
                let result = npp_core::db::prepare(&options.input_dir)
                    .map(|_| ())
                    .map_err(|e| format!("{e:#}"));
                let _ = sender.send(WorkerEvent::Prepared(result));
            });
        } else {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string();
            let log_path = workspace().join("logs").join(format!("{id}.log"));
            let total = self
                .manifest
                .as_ref()
                .map(|m| m.selected)
                .unwrap_or_default();
            self.show_logs = false;
            if self.jobs.len() >= 30 {
                self.jobs.drain(..self.jobs.len() - 29);
            }
            self.running = Some(self.jobs.len());
            self.selected_job = self.running;
            self.jobs.push(Job {
                id,
                label: chrono::Local::now().format("%d.%m.%Y · %H:%M").to_string(),
                options: options.clone(),
                status: "Выполняется".into(),
                completed: 0,
                total,
                success: 0,
                failed: 0,
                warnings: 0,
                result: None,
                error: None,
                logs: VecDeque::new(),
                log_path: log_path.clone(),
                elapsed_ms: 0,
                started: Some(Instant::now()),
            });
            if let Err(e) = Job::save(&self.jobs) {
                self.error(format!("Не удалось сохранить историю: {e}"), cx);
            }
            std::thread::spawn(move || {
                let result = (|| -> Result<BatchResult> {
                    if let Some(parent) = log_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let file = std::sync::Mutex::new(fs::File::create(log_path)?);
                    batch::process(&options, &|event| {
                        if let Progress::Log(line) = &event {
                            let _ = writeln!(file.lock().unwrap(), "{line}");
                        }
                        let _ = sender.send(WorkerEvent::Progress(event));
                    })
                })()
                .map_err(|e| format!("{e:#}"));
                let _ = sender.send(WorkerEvent::Done(result));
            });
        }
        cx.notify();
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.inspection_due.is_some_and(|due| Instant::now() >= due) && !self.busy {
            self.inspect(cx);
            changed = true;
        }
        if let Some((revision, manifest, catalog)) =
            self.inspection.as_ref().and_then(|r| r.try_recv().ok())
        {
            self.inspection = None;
            if revision == self.revision {
                self.manifest = Some(manifest);
                self.catalog = catalog;
                self.results.clear();
                self.groups.clear();
                self.expanded_frame = None;
                self.searched = false;
                self.page = 0;
                changed = true;
            }
        }
        let events: Vec<_> = self
            .receiver
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        changed |= !events.is_empty();
        for event in events {
            match event {
                WorkerEvent::Progress(event) => {
                    if let Some(job) = self.running.and_then(|i| self.jobs.get_mut(i)) {
                        match event {
                            Progress::Log(line) => {
                                if line.starts_with("WARNING ") {
                                    job.warnings += 1;
                                }
                                job.logs.push_back(line);
                                if job.logs.len() > 300 {
                                    job.logs.pop_front();
                                }
                            }
                            Progress::Counts {
                                completed,
                                total,
                                success,
                                failed,
                            } => {
                                job.completed = job.completed.max(completed);
                                job.total = total;
                                job.success = job.success.max(success);
                                job.failed = job.failed.max(failed);
                            }
                        }
                    }
                }
                WorkerEvent::Done(result) => {
                    self.busy = false;
                    self.receiver = None;
                    if let Some(job) = self.running.take().and_then(|i| self.jobs.get_mut(i)) {
                        job.elapsed_ms = job.elapsed().as_millis() as u64;
                        job.started = None;
                        match result {
                            Ok(result) => {
                                job.status = if result.failed == 0 {
                                    "Готово"
                                } else {
                                    "С ошибками"
                                }
                                .into();
                                job.result = Some(result);
                            }
                            Err(error) => {
                                job.status = "Ошибка".into();
                                job.error = Some(error.clone());
                                self.message = error;
                                self.message_error = true;
                            }
                        }
                    }
                    if let Err(e) = Job::save(&self.jobs) {
                        self.error(format!("Не удалось сохранить историю: {e}"), cx);
                    }
                    self.schedule_inspection();
                }
                WorkerEvent::Prepared(result) => {
                    self.busy = false;
                    self.preparing = false;
                    self.receiver = None;
                    match result {
                        Ok(()) => {
                            self.message = "CSV готовы. Можно создавать паспорта.".into();
                            self.message_error = false;
                        }
                        Err(error) => {
                            self.message = error;
                            self.message_error = true;
                        }
                    }
                    self.schedule_inspection();
                }
            }
        }
        if self.busy && self.last_tick.elapsed() >= Duration::from_secs(1) {
            self.last_tick = Instant::now();
            changed = true;
        }
        if self.running.is_some() && self.last_history_save.elapsed() >= Duration::from_secs(5) {
            if let Some(job) = self.running.and_then(|i| self.jobs.get_mut(i)) {
                job.elapsed_ms = job.elapsed().as_millis() as u64;
            }
            if let Err(e) = Job::save(&self.jobs) {
                self.error(format!("Не удалось сохранить историю: {e}"), cx);
            }
            self.last_history_save = Instant::now();
        }
        if changed {
            cx.notify();
        }
    }
    fn search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let query = Self::value(&self.query, cx);
        if query.trim().is_empty() {
            self.error("Введите имя подмодели или выберите её в списке.", cx);
            return;
        }
        if let Some(catalog) = &self.catalog {
            self.results = catalog.index.search(&query);
            self.groups.clear();
            self.expanded_frame = None;
            for (i, record) in self.results.iter().enumerate() {
                if i == 0 || record.frame_name != self.results[i - 1].frame_name {
                    self.groups.push(i..i + 1);
                } else if let Some(group) = self.groups.last_mut() {
                    group.end = i + 1;
                }
            }
            self.searched = true;
            self.page = 0;
            self.message.clear();
        } else {
            self.error(
                "В выбранной папке пока нет индекса. Создайте паспорта, затем повторите поиск.",
                cx,
            );
        }
        cx.notify();
    }
    fn choose(&mut self, which: usize, window: &mut Window, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Выберите папку".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = picker.await;
            let _ = this.update_in(cx, |app, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            let input = match which {
                                0 => app.input.clone(),
                                1 => app.svg.clone(),
                                _ => app.output.clone(),
                            };
                            input.update(cx, |input, cx| {
                                input.set_value(path.to_string_lossy().to_string(), window, cx)
                            });
                            if let Err(e) = app.persist(cx) {
                                app.error(e.to_string(), cx);
                            }
                            app.schedule_inspection();
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => app.error(error.to_string(), cx),
                    Err(error) => app.error(error.to_string(), cx),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn reveal(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.exists() {
            cx.reveal_path(&path);
        } else {
            self.error(format!("Файл или папка не найдены: {}", path.display()), cx);
        }
    }
}
impl Focusable for Desktop {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for Desktop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.shell(cx)
    }
}
pub fn run() -> Result<()> {
    Application::new()
        .with_assets(crate::assets::Assets)
        .run(|cx| {
            gpui_component::init(cx);
            gpui_component::set_locale("ru");
            let primary = if cfg!(target_os = "macos") {
                "cmd"
            } else {
                "ctrl"
            };
            cx.bind_keys([
                KeyBinding::new(&format!("{primary}-1"), CreatePassports, Some("Desktop")),
                KeyBinding::new(&format!("{primary}-2"), FindFrame, Some("Desktop")),
                KeyBinding::new(&format!("{primary}-3"), ShowRuns, Some("Desktop")),
            ]);
            let bounds = Bounds::centered(None, size(px(1240.0), px(820.0)), cx);
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(1020.0), px(680.0))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Паспорта кадров".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| Desktop::new(window, cx));
                    view.read(cx).focus_handle(cx).focus(window);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            );
            if let Err(error) = result {
                eprintln!("Не удалось открыть окно: {error:#}");
                cx.quit();
            }
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
    Ok(())
}
