use anyhow::{Context as _, Result};
use directories::{ProjectDirs, UserDirs};
use gpui::{Context, prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, Root, Theme, ThemeMode,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};
use npp_core::batch::{self, BatchResult, Options, Progress, SearchIndex, SearchRecord};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    input_dir: String,
    svg_dir: String,
    output_dir: String,
    dark: bool,
}

fn settings_path() -> PathBuf {
    ProjectDirs::from("local", "npp", "NppToDocx")
        .map(|p| p.config_dir().join("settings.json"))
        .unwrap_or_else(|| PathBuf::from("settings.json"))
}

fn workspace() -> PathBuf {
    UserDirs::new()
        .and_then(|p| p.document_dir().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("npp_to_docx")
}

impl Settings {
    fn load() -> Self {
        let mut settings: Self = fs::read(settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if settings.input_dir.is_empty() {
            settings.input_dir = workspace().join("input").to_string_lossy().into();
        }
        if settings.output_dir.is_empty() {
            settings.output_dir = workspace().join("output").to_string_lossy().into();
        }
        settings
    }
    fn save(&self) -> Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

enum WorkerEvent {
    Progress(Progress),
    Done(std::result::Result<BatchResult, String>),
    Prepared(std::result::Result<(), String>),
}

struct Job {
    id: String,
    options: Options,
    status: String,
    completed: usize,
    total: usize,
    success: usize,
    failed: usize,
    result: Option<BatchResult>,
    error: Option<String>,
    logs: Vec<String>,
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
    tab: usize,
    busy: bool,
    message: String,
    jobs: Vec<Job>,
    active: Option<usize>,
    receiver: Option<mpsc::Receiver<WorkerEvent>>,
    index: Option<SearchIndex>,
    results: Vec<SearchRecord>,
    page: usize,
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
        Theme::change(
            if settings.dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            Some(window),
            cx,
        );
        let mut app = Self {
            input: field(
                settings.input_dir.clone(),
                "Папка с DMP или CSV",
                window,
                cx,
            ),
            svg: field(
                settings.svg_dir.clone(),
                "Необязательно: отдельная папка кадров",
                window,
                cx,
            ),
            output: field(settings.output_dir.clone(), "Готовые документы", window, cx),
            concurrency: field(
                batch::default_concurrency().to_string(),
                "Например, 2",
                window,
                cx,
            ),
            filter: field(String::new(), "Например, 4UJ", window, cx),
            limit: field(String::new(), "Все кадры", window, cx),
            query: field(String::new(), "Например, DS_ana.svg", window, cx),
            settings,
            tab: 0,
            busy: false,
            message: String::new(),
            jobs: Vec::new(),
            active: None,
            receiver: None,
            index: None,
            results: Vec::new(),
            page: 0,
        };
        app.reload_search(cx);
        cx.observe(&app.query, |_, _, cx| cx.notify()).detach();
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
        self.settings.save()
    }

    fn options(&self, cx: &App) -> Result<Options> {
        let svg = Self::value(&self.svg, cx);
        let filter = Self::value(&self.filter, cx);
        let limit = Self::value(&self.limit, cx);
        let concurrency = Self::value(&self.concurrency, cx)
            .parse::<usize>()
            .context("Введите число параллельных обработок от 1 до 256")?;
        anyhow::ensure!(
            (1..=256).contains(&concurrency),
            "Число параллельных обработок должно быть от 1 до 256"
        );
        let limit = if limit.trim().is_empty() {
            None
        } else {
            let limit = limit
                .trim()
                .parse::<usize>()
                .context("Лимит должен быть положительным числом")?;
            anyhow::ensure!(limit > 0, "Лимит должен быть положительным числом");
            Some(limit)
        };
        Ok(Options {
            input_dir: Self::value(&self.input, cx).into(),
            svg_dir: if svg.trim().is_empty() {
                None
            } else {
                Some(svg.into())
            },
            output_dir: Self::value(&self.output, cx).into(),
            concurrency,
            r#match: if filter.is_empty() {
                None
            } else {
                Some(filter)
            },
            limit,
        })
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
                self.message = e.to_string();
                cx.notify();
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.busy = true;
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
            self.active = Some(self.jobs.len());
            self.jobs.push(Job {
                id: id.clone(),
                options: options.clone(),
                status: "Выполняется".into(),
                completed: 0,
                total: 0,
                success: 0,
                failed: 0,
                result: None,
                error: None,
                logs: Vec::new(),
            });
            std::thread::spawn(move || {
                let logs_dir = workspace().join("logs");
                let result = (|| -> Result<BatchResult> {
                    fs::create_dir_all(logs_dir.clone())?;
                    let file = std::sync::Mutex::new(fs::File::create(
                        logs_dir.join(format!("{id}.log")),
                    )?);
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
        let events: Vec<_> = self
            .receiver
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        if events.is_empty() {
            return;
        }
        for event in events {
            match event {
                WorkerEvent::Progress(event) => {
                    if let Some(job) = self.jobs.last_mut() {
                        match event {
                            Progress::Log(line) => {
                                job.logs.push(line);
                                if job.logs.len() > 1000 {
                                    job.logs.remove(0);
                                }
                            }
                            Progress::Counts {
                                completed,
                                total,
                                success,
                                failed,
                            } => {
                                // Worker messages can arrive out of order.
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
                    if let Some(job) = self.jobs.last_mut() {
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
                            }
                        }
                    }
                    self.reload_search(cx);
                }
                WorkerEvent::Prepared(result) => {
                    self.busy = false;
                    self.receiver = None;
                    self.message = result
                        .map(|_| "База данных подготовлена".to_string())
                        .unwrap_or_else(|e| e);
                }
            }
        }
        cx.notify();
    }

    fn reload_search(&mut self, cx: &App) {
        self.index = SearchIndex::read(&PathBuf::from(Self::value(&self.output, cx))).ok();
        self.results.clear();
        self.page = 0;
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        match SearchIndex::read(&PathBuf::from(Self::value(&self.output, cx))) {
            Ok(index) => {
                self.results = index.search(&Self::value(&self.query, cx));
                self.index = Some(index);
                self.page = 0;
                self.message.clear();
            }
            Err(error) => {
                self.message = error.to_string();
                self.index = None;
                self.results.clear();
            }
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
                            if let Err(error) = app.persist(cx) {
                                app.message = error.to_string();
                            }
                            if which == 2 {
                                app.reload_search(cx);
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => app.message = error.to_string(),
                    Err(error) => app.message = error.to_string(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn directory(
        &self,
        label: &str,
        input: &Entity<InputState>,
        which: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(label.to_string())
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(input).disabled(self.busy)))
                    .child(
                        Button::new(("choose", which))
                            .label("Выбрать")
                            .disabled(self.busy)
                            .on_click(
                                cx.listener(move |app, _, window, cx| {
                                    app.choose(which, window, cx)
                                }),
                            ),
                    ),
            )
    }

    fn home(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active = self.active.and_then(|i| self.jobs.get(i));
        let mut body = div()
            .flex()
            .flex_col()
            .gap_5()
            .child(self.directory("Входная папка с базой", &self.input, 0, cx))
            .child(self.directory("Папка SVG (если отличается от входной)", &self.svg, 1, cx))
            .child(self.directory("Выходная папка", &self.output, 2, cx))
            .child(
                div().flex().gap_4().children([
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("Обработок одновременно")
                        .child(Input::new(&self.concurrency).disabled(self.busy)),
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("Совпадение в имени файла")
                        .child(Input::new(&self.filter).disabled(self.busy)),
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("Лимит кадров")
                        .child(Input::new(&self.limit).disabled(self.busy)),
                ]),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        Button::new("prepare")
                            .label("Подготовить базу данных")
                            .disabled(self.busy)
                            .on_click(cx.listener(|app, _, _, cx| app.start(true, cx))),
                    )
                    .child(
                        Button::new("convert")
                            .primary()
                            .label(if self.busy {
                                "Обработка…"
                            } else {
                                "Запустить обработку"
                            })
                            .disabled(self.busy)
                            .on_click(cx.listener(|app, _, _, cx| app.start(false, cx))),
                    ),
            )
            .child(div().h(px(1.0)).bg(theme.border));
        if let Some(job) = active {
            let total = job.total.max(1) as f32;
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "{} · {}/{} · успешно {} · ошибок {}",
                        job.status, job.completed, job.total, job.success, job.failed
                    ))
                    .child(
                        div()
                            .flex()
                            .h(px(10.0))
                            .w_full()
                            .bg(theme.muted)
                            .rounded_md()
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(job.success as f32 / total))
                                    .bg(rgb(0x29966c)),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(job.failed as f32 / total))
                                    .bg(rgb(0xd62828)),
                            ),
                    )
                    .children(job.result.as_ref().map(|r| div().child(r.summary.clone())))
                    .children(
                        job.error
                            .as_ref()
                            .map(|e| div().text_color(rgb(0xd62828)).child(e.clone())),
                    )
                    .child(
                        div()
                            .id("logs")
                            .max_h(px(200.0))
                            .overflow_y_scroll()
                            .p_3()
                            .bg(theme.muted)
                            .text_sm()
                            .children(
                                job.logs
                                    .iter()
                                    .rev()
                                    .take(80)
                                    .map(|line| div().child(line.clone())),
                            ),
                    ),
            );
        }
        body.child(div().text_lg().child("История обработок"))
            .children(self.jobs.iter().enumerate().rev().map(|(i, job)| {
                Button::new(("job", i))
                    .label(format!(
                        "{} · {} · {}",
                        job.id,
                        job.status,
                        job.options.input_dir.display()
                    ))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.active = Some(i);
                        cx.notify();
                    }))
            }))
    }

    fn search_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let query = Self::value(&self.query, cx);
        let suggestions = self
            .index
            .as_ref()
            .map(|i| i.submodels(&query))
            .unwrap_or_default();
        let pages = self.results.len().div_ceil(20).max(1);
        let mut body = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Поиск по подмодели")
            .child(
                self.index
                    .as_ref()
                    .map(|i| {
                        format!(
                            "Индекс готов: {} записей, {} подмоделей",
                            i.records.len(),
                            i.submodels("").len()
                        )
                    })
                    .unwrap_or("Запустите обработку для создания индекса поиска".into()),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(div().flex_1().child(Input::new(&self.query)))
                    .child(
                        Button::new("search")
                            .primary()
                            .label("Показать видеокадры")
                            .on_click(cx.listener(|app, _, _, cx| app.search(cx))),
                    ),
            )
            .child(
                div().flex().flex_wrap().gap_2().children(
                    suggestions
                        .into_iter()
                        .take(12)
                        .enumerate()
                        .map(|(i, (name, count))| {
                            Button::new(("suggestion", i))
                                .label(format!("{name} ({count})"))
                                .on_click(cx.listener(move |app, _, window, cx| {
                                    app.query.update(cx, |input, cx| {
                                        input.set_value(name.clone(), window, cx)
                                    });
                                    app.search(cx);
                                }))
                        }),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().w(px(200.0)).child("Видеокадр"))
                    .child(div().w(px(80.0)).child("№"))
                    .child(div().w(px(220.0)).child("KKS"))
                    .child(div().flex_1().child("Описание")),
            );
        for record in self.results.iter().skip(self.page * 20).take(20) {
            body = body.child(
                div()
                    .flex()
                    .gap_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().w(px(200.0)).child(record.frame_name.clone()))
                    .child(div().w(px(80.0)).child(record.marker_index.to_string()))
                    .child(
                        div()
                            .w(px(220.0))
                            .child(record.kks.clone().unwrap_or(record.title.clone())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(record.description.clone().unwrap_or("—".into())),
                    ),
            );
        }
        if self.results.is_empty() {
            body = body.child("Совпадений нет. Выберите подмодель и выполните поиск.");
        }
        body.child(
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(
                    Button::new("previous")
                        .label("Назад")
                        .disabled(self.page == 0)
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.page = app.page.saturating_sub(1);
                            cx.notify();
                        })),
                )
                .child(format!(
                    "Страница {} из {pages} · {} записей",
                    self.page + 1,
                    self.results.len()
                ))
                .child(
                    Button::new("next")
                        .label("Вперёд")
                        .disabled(self.page + 1 >= pages)
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.page += 1;
                            cx.notify();
                        })),
                ),
        )
    }
}

impl Render for Desktop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().background;
        let foreground = cx.theme().foreground;
        div()
            .id("desktop")
            .size_full()
            .overflow_y_scroll()
            .bg(background)
            .text_color(foreground)
            .p_6()
            .child(
                div()
                    .max_w(px(1200.0))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().text_2xl().flex_1().child("npp_to_docx"))
                            .child(Button::new("home").label("Главная").on_click(cx.listener(
                                |app, _, _, cx| {
                                    app.tab = 0;
                                    cx.notify();
                                },
                            )))
                            .child(
                                Button::new("search-tab")
                                    .label("Поиск")
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.tab = 1;
                                        app.reload_search(cx);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("theme")
                                    .label(if self.settings.dark {
                                        "Светлая тема"
                                    } else {
                                        "Тёмная тема"
                                    })
                                    .on_click(cx.listener(|app, _, window, cx| {
                                        app.settings.dark = !app.settings.dark;
                                        Theme::change(
                                            if app.settings.dark {
                                                ThemeMode::Dark
                                            } else {
                                                ThemeMode::Light
                                            },
                                            Some(window),
                                            cx,
                                        );
                                        if let Err(error) = app.persist(cx) {
                                            app.message = error.to_string();
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .children(
                        (!self.message.is_empty())
                            .then(|| div().p_3().bg(cx.theme().muted).child(self.message.clone())),
                    )
                    .child(if self.tab == 0 {
                        self.home(cx).into_any_element()
                    } else {
                        self.search_tab(cx).into_any_element()
                    }),
            )
    }
}

pub fn run() -> Result<()> {
    Application::new().run(|cx| {
        gpui_component::init(cx);
        let bounds = Bounds::centered(None, size(px(1100.0), px(850.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.0), px(600.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("NppToDocx".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Desktop::new(window, cx));
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
