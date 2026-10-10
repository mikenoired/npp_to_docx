use super::*;

fn short_label(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.into()
    } else {
        format!("{}…", text.chars().take(limit - 1).collect::<String>())
    }
}
fn surface(p: Palette) -> Div {
    div()
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .rounded(px(8.0))
}
fn hint(text: impl Into<SharedString>, p: Palette) -> Div {
    div()
        .text_size(px(12.0))
        .text_color(p.muted)
        .child(text.into())
}
fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(16.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}
fn badge(text: impl Into<SharedString>, color: Hsla, p: Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded(px(4.0))
        .bg(p.inset)
        .text_size(px(11.0))
        .text_color(color)
        .child(text.into())
}
fn empty(icon: IconName, title: &str, detail: &str, p: Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .child(
            div()
                .p_3()
                .rounded(px(10.0))
                .bg(p.inset)
                .text_color(p.accent)
                .child(Icon::new(icon).size(px(24.0))),
        )
        .child(heading(title.to_string()))
        .child(
            div()
                .max_w(px(400.0))
                .text_center()
                .text_size(px(13.0))
                .text_color(p.muted)
                .child(detail.to_string()),
        )
}

impl Desktop {
    pub(super) fn shell(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        div()
            .id("desktop")
            .key_context("Desktop")
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|app, _: &CreatePassports, window, cx| app.navigate(0, window, cx)),
            )
            .on_action(cx.listener(|app, _: &FindFrame, window, cx| app.navigate(1, window, cx)))
            .on_action(cx.listener(|app, _: &ShowRuns, window, cx| app.navigate(2, window, cx)))
            .size_full()
            .flex()
            .bg(p.canvas)
            .text_color(p.ink)
            .text_size(px(14.0))
            .child(self.sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(self.page_header(cx))
                    .when(!self.message.is_empty(), |view| {
                        view.child(
                            div()
                                .mx_6()
                                .mb_4()
                                .p_3()
                                .flex()
                                .items_start()
                                .gap_3()
                                .bg(p.surface)
                                .border_1()
                                .border_color(if self.message_error { p.danger } else { p.good })
                                .rounded(px(6.0))
                                .child(
                                    Icon::new(if self.message_error {
                                        IconName::TriangleAlert
                                    } else {
                                        IconName::CircleCheck
                                    })
                                    .text_color(if self.message_error { p.danger } else { p.good }),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(13.0))
                                        .child(self.message.clone()),
                                )
                                .child(
                                    Button::new("dismiss-message")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close)
                                        .tooltip("Закрыть сообщение")
                                        .on_click(cx.listener(|app, _, _, cx| {
                                            app.message.clear();
                                            cx.notify();
                                        })),
                                ),
                        )
                    })
                    .child(match self.tab {
                        1 => self.search_page(cx).into_any_element(),
                        2 => self.history_page(cx).into_any_element(),
                        _ => self.home(cx).into_any_element(),
                    })
                    .child(
                        div()
                            .h(px(30.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .border_t_1()
                            .border_color(p.line)
                            .text_size(px(11.0))
                            .text_color(p.muted)
                            .child("Паспорта SVG-кадров · KKS · Подмодели")
                            .child(if self.busy {
                                "Обработка идёт в фоне"
                            } else {
                                "Готов к работе"
                            }),
                    ),
            )
    }

    fn sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        let modifier = if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl+"
        };
        let mut nav = div().flex().flex_col().gap_1();
        for (tab, icon, label) in [
            (0, IconName::File, "Создать паспорта"),
            (1, IconName::Search, "Найти кадр"),
            (2, IconName::BookOpen, "Запуски"),
        ] {
            nav = nav.child(
                Button::new(("nav", tab))
                    .ghost()
                    .w_full()
                    .h(px(38.0))
                    .justify_start()
                    .icon(icon)
                    .label(label)
                    .tooltip(format!("{label} · {modifier}{}", tab + 1))
                    .when(self.tab == tab, |b| b.bg(p.selected).text_color(p.accent))
                    .on_click(cx.listener(move |app, _, window, cx| app.navigate(tab, window, cx))),
            );
        }
        div()
            .w(px(192.0))
            .flex_none()
            .h_full()
            .bg(p.surface)
            .border_r_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pt_3()
                    .pb_8()
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(38.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(p.accent)
                            .rounded(px(5.0))
                            .text_color(p.accent)
                            .child(Icon::new(IconName::File).size(px(20.0))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_family(theme::display_font())
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Паспорта"),
                            )
                            .child(
                                div()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_size(px(10.0))
                                    .text_color(p.muted)
                                    .child("SVG → DOCX"),
                            ),
                    ),
            )
            .child(hint("РАБОЧАЯ ОБЛАСТЬ", p).mb_3().text_size(px(10.0)))
            .child(nav)
            .when(self.busy, |v| {
                v.child(
                    div()
                        .mt_5()
                        .px_3()
                        .py_3()
                        .bg(p.inset)
                        .rounded(px(6.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_color(p.accent)
                                .text_size(px(12.0))
                                .child(Icon::new(IconName::LoaderCircle).small())
                                .child(if self.preparing {
                                    "Подготовка CSV"
                                } else {
                                    "Создание паспортов"
                                }),
                        )
                        .when_some(self.running.and_then(|i| self.jobs.get(i)), |v, job| {
                            v.child(
                                hint(
                                    format!(
                                        "{} / {} кадров",
                                        count(job.completed),
                                        count(job.total)
                                    ),
                                    p,
                                )
                                .mt_2(),
                            )
                        }),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .border_t_1()
                    .border_color(p.line)
                    .pt_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Button::new("theme")
                            .ghost()
                            .small()
                            .justify_start()
                            .icon(if self.settings.dark {
                                IconName::Sun
                            } else {
                                IconName::Moon
                            })
                            .label(if self.settings.dark {
                                "Светлая тема"
                            } else {
                                "Тёмная тема"
                            })
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.settings.dark = !app.settings.dark;
                                theme::apply(app.settings.dark, window, cx);
                                if let Err(e) = app.persist(cx) {
                                    app.error(e.to_string(), cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(hint(format!("Версия {}", env!("CARGO_PKG_VERSION")), p).px_2())
                    .when(cfg!(debug_assertions), |v| {
                        v.child(badge("Отладочная сборка", p.warning, p))
                    }),
            )
    }

    fn page_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        let (title, description) = match self.tab {
            1 => (
                "Найти кадр",
                "Кадры, в которых используется выбранная подмодель",
            ),
            2 => ("Журнал запусков", "Результаты обработки и полные логи"),
            _ => ("Новый пакет", "Создание паспортов из SVG и базы описаний"),
        };
        let status = if self.tab == 0 {
            match &self.manifest {
                Some(m) if m.ready() => badge("Источники готовы", p.good, p),
                Some(_) => badge("Проверьте источники", p.warning, p),
                None => badge(
                    if self.options(cx).is_err() {
                        "Проверьте параметры"
                    } else {
                        "Проверка источников…"
                    },
                    p.muted,
                    p,
                ),
            }
        } else if self.tab == 1 {
            badge(
                if self.catalog.is_some() {
                    "Индекс готов"
                } else {
                    "Нет индекса"
                },
                p.muted,
                p,
            )
        } else {
            badge(format!("{} запусков", count(self.jobs.len())), p.muted, p)
        };
        div()
            .flex_none()
            .px_6()
            .pt_6()
            .pb_5()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .font_family(theme::display_font())
                            .text_size(px(24.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(hint(description, p)),
            )
            .child(status)
    }

    fn directory(
        &self,
        label: &str,
        detail: &str,
        input: &Entity<InputState>,
        which: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(13.0))
                            .child(label.to_string()),
                    )
                    .child(hint(detail.to_string(), p)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(Input::new(input).small().disabled(self.busy)),
                    )
                    .child(
                        Button::new(("choose", which))
                            .small()
                            .icon(IconName::FolderOpen)
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
        let p = Palette::new(self.settings.dark);
        let can_start = !self.busy && self.manifest.as_ref().is_some_and(Manifest::ready);
        let can_prepare = !self.busy
            && self
                .manifest
                .as_ref()
                .is_some_and(|m| m.databases.iter().all(|db| db.format.is_some()));
        let filter = Self::value(&self.filter, cx);
        let limit = Self::value(&self.limit, cx);
        let current = self.running.or_else(|| self.jobs.len().checked_sub(1));
        let mut advanced = div().flex().flex_col().gap_3().child(
            Button::new("advanced")
                .ghost()
                .small()
                .justify_start()
                .icon(if self.advanced {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .label("Параметры обработки")
                .on_click(cx.listener(|app, _, _, cx| {
                    app.advanced = !app.advanced;
                    cx.notify();
                })),
        );
        if self.advanced {
            advanced = advanced
                .child(
                    div().flex().gap_3().children([
                        div()
                            .w(px(100.0))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(hint("Одновременно", p))
                            .child(Input::new(&self.concurrency).small().disabled(self.busy)),
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(hint("Имя содержит", p))
                            .child(
                                Input::new(&self.filter)
                                    .small()
                                    .cleanable(true)
                                    .disabled(self.busy),
                            ),
                        div()
                            .w(px(110.0))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(hint("Лимит кадров", p))
                            .child(Input::new(&self.limit).small().disabled(self.busy)),
                    ]),
                )
                .child(hint(
                    "Фильтр применяется до лимита. Пустой лимит — обработать все кадры.",
                    p,
                ));
        } else {
            advanced = advanced.child(
                hint(
                    format!(
                        "Одновременно: {} · {} · {}",
                        Self::value(&self.concurrency, cx),
                        if filter.is_empty() {
                            "Все имена".into()
                        } else {
                            format!("Фильтр: {filter}")
                        },
                        if limit.is_empty() {
                            "Без лимита".into()
                        } else {
                            format!("Лимит: {limit}")
                        }
                    ),
                    p,
                )
                .px_2(),
            );
        }
        let controls = surface(p).flex().flex_col().p_5().gap_4()
            .child(div().flex().items_center().justify_between()
                .child(heading("Исходные данные"))
                .child(Icon::new(IconName::FolderOpen).text_color(p.muted)))
            .child(self.directory("База описаний", "DMP или CSV", &self.input, 0, cx))
            .child(self.directory("SVG-кадры", "Отдельная папка — по необходимости", &self.svg, 1, cx))
            .child(div().h(px(1.0)).bg(p.line))
            .child(self.directory("Готовые паспорта", "DOCX + поисковый индекс", &self.output, 2, cx))
            .child(advanced)
            .when_some(self.options(cx).err(), |v, error| v.child(div().text_size(px(12.0)).text_color(p.danger).child(error.to_string())))
            .child(div().flex().items_center().justify_between().gap_3().pt_3().border_t_1().border_color(p.line)
                .child(Button::new("prepare").ghost().small().label(if self.preparing { "Подготовка CSV…" } else { "Подготовить CSV" })
                    .tooltip("DMP подготавливаются автоматически при создании паспортов. Здесь можно подготовить CSV отдельно.")
                    .disabled(!can_prepare).on_click(cx.listener(|app, _, _, cx| app.start(true, cx))))
                .child(Button::new("convert").primary().icon(IconName::ArrowRight)
                    .label(if self.busy && !self.preparing { "Создание паспортов…" } else { "Создать паспорта" })
                    .disabled(!can_start).on_click(cx.listener(|app, _, _, cx| app.start(false, cx)))));
        let mut page = div()
            .id("home-scroll")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px_6()
            .pb_6()
            .flex()
            .flex_col()
            .gap_5();
        if cfg!(debug_assertions) {
            page = page.child(
                surface(p)
                    .p_3()
                    .border_color(p.warning)
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(Icon::new(IconName::TriangleAlert).text_color(p.warning))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child("Отладочная сборка обрабатывает схемы значительно медленнее.")
                            .child(hint(
                                "Для работы: cargo run --release --locked -p npp-to-docx",
                                p,
                            )),
                    ),
            );
        }
        page.child(
            div()
                .flex()
                .items_start()
                .gap_5()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap_5()
                        .child(controls)
                        .when_some(current, |v, i| v.child(self.job_panel(i, false, cx))),
                )
                .child(div().w(px(284.0)).flex_none().child(self.package(cx))),
        )
    }

    fn package(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        let mut panel = surface(p).flex().flex_col().child(
            div()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .child(heading("Состав пакета"))
                .child(badge("SVG → DOCX", p.accent, p)),
        );
        if let Some(m) = &self.manifest {
            panel = panel.child(
                div()
                    .px_4()
                    .py_4()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(px(28.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(count(m.selected)),
                    )
                    .child(hint("паспортов к созданию", p)),
            );
            for db in &m.databases {
                panel = panel.child(
                    div()
                        .px_4()
                        .py_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(if db.format.is_some() {
                                IconName::CircleCheck
                            } else {
                                IconName::TriangleAlert
                            })
                            .small()
                            .text_color(if db.format.is_some() {
                                p.good
                            } else {
                                p.warning
                            }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(px(11.0))
                                .child(db.name),
                        )
                        .child(badge(
                            db.format.unwrap_or("Нет файла"),
                            if db.format.is_some() {
                                p.muted
                            } else {
                                p.warning
                            },
                            p,
                        )),
                );
            }
            if let Some(error) = &m.error {
                panel = panel.child(
                    div()
                        .px_4()
                        .py_3()
                        .text_size(px(12.0))
                        .text_color(p.danger)
                        .child(error.clone()),
                );
            } else if m.databases.iter().any(|db| db.format.is_none()) {
                panel = panel.child(
                    hint("Добавьте отсутствующую базу в выбранную входную папку.", p)
                        .px_4()
                        .py_3(),
                );
            } else if m.selected == 0 {
                panel = panel.child(
                    hint("Нет подходящих SVG. Проверьте папку и фильтр имени.", p)
                        .px_4()
                        .py_3(),
                );
            } else if m.databases.iter().any(|db| db.format == Some("DMP")) {
                panel = panel.child(
                    hint("CSV будут подготовлены автоматически.", p)
                        .px_4()
                        .py_3(),
                );
            }
            panel = panel.child(
                div()
                    .mt_3()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(p.line)
                    .child(hint(format!("Кадры в источнике · {}", count(m.matched)), p))
                    .child(
                        div()
                            .mt_1()
                            .truncate()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(px(11.0))
                            .text_color(p.muted)
                            .child(format!(
                                "Папка: {}",
                                m.svg_dir.file_name().unwrap_or_default().to_string_lossy()
                            )),
                    ),
            );
            for name in m.frames.iter().take(5) {
                panel = panel.child(
                    div()
                        .px_4()
                        .py_2()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(p.accent)
                                .font_family(cx.theme().mono_font_family.clone())
                                .child("SVG"),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.0))
                                .font_family(cx.theme().mono_font_family.clone())
                                .child(name.clone()),
                        ),
                );
            }
            if m.matched > 5 {
                panel = panel.child(
                    hint(format!("и ещё {} кадров", count(m.matched - 5)), p)
                        .px_4()
                        .py_2(),
                );
            }
            if !m.output_exists {
                panel = panel.child(
                    hint("Выходная папка будет создана при запуске.", p)
                        .px_4()
                        .py_3(),
                );
            }
        } else {
            panel = panel.child(div().p_4().flex().flex_col().gap_3().child(hint(
                if self.options(cx).is_err() {
                    "Исправьте параметры обработки."
                } else {
                    "Проверяем папки и базы…"
                },
                p,
            )));
        }
        panel.child(
            div()
                .p_4()
                .mt_3()
                .border_t_1()
                .border_color(p.line)
                .bg(p.inset)
                .flex()
                .flex_col()
                .gap_2()
                .child(hint("В КАЖДОМ ПАСПОРТЕ", p).text_size(px(10.0)))
                .child(
                    div()
                        .text_size(px(12.0))
                        .child("Схема с номерами элементов"),
                )
                .child(div().text_size(px(12.0)).child("KKS, описания и подмодели"))
                .child(hint("Поисковый индекс сохраняется рядом.", p)),
        )
    }

    fn job_panel(&self, index: usize, detailed: bool, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        let job = &self.jobs[index];
        let running = self.running == Some(index);
        let status_color = if running {
            p.accent
        } else if job.error.is_some() || job.failed > 0 {
            p.danger
        } else {
            p.good
        };
        let total = job.total.max(1) as f32;
        let output = job.options.output_dir.clone();
        let log = job.log_path.clone();
        let elapsed = job.elapsed();
        let mut panel = surface(p)
            .flex()
            .flex_col()
            .p_5()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(heading(if detailed {
                                "Результат запуска"
                            } else {
                                "Последний запуск"
                            }))
                            .child(badge(job.status.clone(), status_color, p)),
                    )
                    .child(hint(job.label.clone(), p)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(px(14.0))
                            .child(format!(
                                "{} / {} кадров",
                                count(job.completed),
                                count(job.total)
                            )),
                    )
                    .child(hint(format!("Время: {}", duration(elapsed)), p)),
            )
            .child(
                div()
                    .flex()
                    .h(px(6.0))
                    .w_full()
                    .rounded(px(3.0))
                    .overflow_hidden()
                    .bg(p.inset)
                    .child(
                        div()
                            .h_full()
                            .w(relative((job.success as f32 / total).min(1.0)))
                            .bg(p.good),
                    )
                    .child(
                        div()
                            .h_full()
                            .w(relative((job.failed as f32 / total).min(1.0)))
                            .bg(p.danger),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_5()
                    .flex_wrap()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.good)
                            .child(format!("Создано: {}", count(job.success))),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if job.failed > 0 { p.danger } else { p.muted })
                            .child(format!("Ошибок: {}", count(job.failed))),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if job.warnings > 0 { p.warning } else { p.muted })
                            .child(format!("Без подмоделей: {} кадров", count(job.warnings))),
                    ),
            );
        if running && job.completed > 0 && elapsed.as_secs() >= 3 {
            let remaining = elapsed.as_secs_f64() / job.completed as f64
                * job.total.saturating_sub(job.completed) as f64;
            panel = panel.child(hint(
                format!(
                    "Примерно осталось {}",
                    duration(Duration::from_secs_f64(remaining))
                ),
                p,
            ));
        }
        if let Some(result) = &job.result {
            panel = panel.child(hint(
                format!(
                    "Элементов: {} · Различий title / KKS: {}",
                    count(result.total_markers),
                    count(result.total_mismatches)
                ),
                p,
            ));
        }
        if let Some(error) = &job.error {
            panel = panel.child(
                div()
                    .text_size(px(13.0))
                    .text_color(p.danger)
                    .child(error.clone()),
            );
        }
        if detailed {
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(p.inset)
                    .rounded(px(5.0))
                    .child(hint("ИСТОЧНИК", p).text_size(px(10.0)))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .child(job.options.input_dir.to_string_lossy().into_owned()),
                    )
                    .child(hint("РЕЗУЛЬТАТ", p).mt_2().text_size(px(10.0)))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .child(job.options.output_dir.to_string_lossy().into_owned()),
                    )
                    .child(hint(
                        format!(
                            "Параллелизм: {} · Фильтр: {} · Лимит: {}",
                            job.options.concurrency,
                            job.options.r#match.as_deref().unwrap_or("нет"),
                            job.options
                                .limit
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "нет".into())
                        ),
                        p,
                    )),
            );
        }
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(
                    Button::new(("reveal-output", index))
                        .small()
                        .icon(IconName::FolderOpen)
                        .label("Показать результаты")
                        .disabled(job.success == 0)
                        .on_click(cx.listener(move |app, _, _, cx| app.reveal(output.clone(), cx))),
                )
                .child(
                    Button::new(("reveal-log", index))
                        .ghost()
                        .small()
                        .label("Полный лог")
                        .icon(IconName::ExternalLink)
                        .on_click(cx.listener(move |app, _, _, cx| app.reveal(log.clone(), cx))),
                )
                .child(
                    Button::new(("toggle-logs", index))
                        .ghost()
                        .small()
                        .label(if self.show_logs {
                            "Скрыть журнал"
                        } else {
                            "Журнал"
                        })
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.show_logs = !app.show_logs;
                            cx.notify();
                        })),
                ),
        );
        if self.show_logs {
            panel = panel.child(
                div()
                    .id(("logs", index))
                    .max_h(px(220.0))
                    .overflow_y_scroll()
                    .p_3()
                    .bg(p.inset)
                    .rounded(px(5.0))
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(11.0))
                    .children(job.logs.iter().rev().take(100).map(|line| {
                        div()
                            .py_1()
                            .text_color(if line.starts_with("FAIL ") {
                                p.danger
                            } else if line.starts_with("WARNING ") {
                                p.warning
                            } else {
                                p.muted
                            })
                            .child(line.clone())
                    })),
            );
        }
        panel
    }

    fn search_page(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        let Some(catalog) = &self.catalog else {
            return div().flex_1().min_h(px(0.0)).px_6().pb_6().child(surface(p).h_full().flex().flex_col().items_center().justify_center()
                .child(empty(IconName::Search, "Пока нет поискового индекса",
                    "Создайте паспорта или выберите папку с готовыми документами и search-index.json.", p))
                .child(Button::new("back-home").primary().label("Перейти к созданию паспортов")
                    .on_click(cx.listener(|app, _, window, cx| app.navigate(0, window, cx))))).into_any_element();
        };
        let query = Self::value(&self.query, cx);
        let needle = query.trim().to_lowercase();
        let models: Vec<_> = catalog
            .submodels
            .iter()
            .filter(|(name, _)| self.searched || name.to_lowercase().contains(&needle))
            .collect();
        let pages = self.groups.len().div_ceil(25).max(1);
        let directory = catalog.directory.clone();
        let mut list = div()
            .id("submodels-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1()
            .p_2();
        for (i, (name, occurrences)) in models.iter().enumerate() {
            let name = (*name).clone();
            list = list.child(
                Button::new(("submodel", i))
                    .ghost()
                    .small()
                    .w_full()
                    .justify_start()
                    .label(format!(
                        "{} · {}",
                        short_label(&name, 21),
                        count(*occurrences)
                    ))
                    .tooltip(name.clone())
                    .text_size(px(12.0))
                    .when(name.eq_ignore_ascii_case(query.trim()), |b| {
                        b.bg(p.selected).text_color(p.accent)
                    })
                    .on_click(cx.listener(move |app, _, window, cx| {
                        app.query
                            .update(cx, |input, cx| input.set_value(name.clone(), window, cx));
                        app.search(window, cx);
                    })),
            );
        }
        if models.is_empty() {
            list = list.child(hint("Подмодели с таким именем не найдены.", p).p_3());
        }
        let sidebar = surface(p)
            .w(px(230.0))
            .flex_none()
            .flex()
            .flex_col()
            .child(
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(heading("Подмодели"))
                    .child(hint(
                        format!(
                            "Модели: {} · Кадры: {}",
                            count(catalog.submodels.len()),
                            count(catalog.frame_count)
                        ),
                        p,
                    )),
            )
            .child(list);
        let mut table = surface(p)
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .p_4()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w(px(0.0)).child(
                            Input::new(&self.query)
                                .prefix(Icon::new(IconName::Search).text_color(p.muted))
                                .cleanable(true),
                        ),
                    )
                    .child(
                        Button::new("search")
                            .primary()
                            .label("Найти")
                            .disabled(query.trim().is_empty())
                            .on_click(cx.listener(|app, _, window, cx| app.search(window, cx))),
                    ),
            )
            .child(div().px_4().pb_3().child(hint(
                if self.searched {
                    format!(
                        "Кадры: {} · Вхождения: {}",
                        count(self.groups.len()),
                        count(self.results.len())
                    )
                } else {
                    "Выберите подмодель слева или введите её имя и нажмите Enter.".into()
                },
                p,
            )));
        if !self.searched {
            table = table.child(div().flex_1().flex().items_center().justify_center().child(
                empty(
                    IconName::Frame,
                    "Выберите подмодель",
                    "Найдите кадр, раскройте его элементы и перейдите к готовому DOCX.",
                    p,
                ),
            ));
        } else if self.results.is_empty() {
            table = table.child(div().flex_1().flex().items_center().justify_center().child(
                empty(
                    IconName::Search,
                    "Совпадений нет",
                    "Проверьте полное имя подмодели, включая расширение .svg.",
                    p,
                ),
            ));
        } else {
            table = table.child(
                div()
                    .flex()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .bg(p.inset)
                    .border_y_1()
                    .border_color(p.line)
                    .text_size(px(11.0))
                    .text_color(p.muted)
                    .child(div().w(px(156.0)).flex_none().child("КАДР"))
                    .child(div().w(px(68.0)).flex_none().child("ЭЛЕМЕНТЫ"))
                    .child(div().flex_1().child("KKS")),
            );
            let mut rows = div()
                .id("search-results")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for (group_index, range) in self.groups.iter().enumerate().skip(self.page * 25).take(25)
            {
                let records = &self.results[range.clone()];
                let frame = &records[0].frame_name;
                let expanded = self.expanded_frame == Some(group_index);
                let mut document = directory.join(frame);
                document.set_extension("docx");
                let preview = records
                    .iter()
                    .take(2)
                    .map(|r| r.kks.as_deref().unwrap_or(&r.title))
                    .collect::<Vec<_>>()
                    .join(", ");
                rows = rows.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(p.line)
                        .bg(if expanded {
                            p.selected
                        } else if group_index % 2 == 1 {
                            p.inset
                        } else {
                            p.surface
                        })
                        .child(
                            div().w(px(156.0)).flex_none().child(
                                Button::new(("frame", group_index))
                                    .ghost()
                                    .xsmall()
                                    .justify_start()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .label(short_label(frame, 20))
                                    .tooltip(format!("{frame} · показать DOCX в папке результатов"))
                                    .text_color(p.accent)
                                    .on_click(cx.listener(move |app, _, _, cx| {
                                        app.reveal(document.clone(), cx)
                                    })),
                            ),
                        )
                        .child(
                            div()
                                .w(px(68.0))
                                .flex_none()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(px(12.0))
                                .child(count(records.len())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(px(11.0))
                                .text_color(p.muted)
                                .child(preview),
                        )
                        .child(
                            Button::new(("expand-frame", group_index))
                                .ghost()
                                .small()
                                .icon(if expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .tooltip("Показать все элементы кадра")
                                .on_click(cx.listener(move |app, _, _, cx| {
                                    app.expanded_frame = if app.expanded_frame == Some(group_index)
                                    {
                                        None
                                    } else {
                                        Some(group_index)
                                    };
                                    cx.notify();
                                })),
                        ),
                );
                if expanded {
                    rows = rows.child(
                        div()
                            .flex()
                            .gap_3()
                            .px_5()
                            .py_2()
                            .bg(p.inset)
                            .text_size(px(10.0))
                            .text_color(p.muted)
                            .child(div().w(px(40.0)).flex_none().child("№"))
                            .child(
                                div()
                                    .w(px(172.0))
                                    .flex_none()
                                    .child("KKS · НАЖМИТЕ ДЛЯ КОПИРОВАНИЯ"),
                            )
                            .child(div().flex_1().child("ОПИСАНИЕ")),
                    );
                    for (record_index, record) in records.iter().enumerate() {
                        let kks = record.kks.clone().unwrap_or(record.title.clone());
                        rows = rows.child(
                            div()
                                .flex()
                                .gap_3()
                                .px_5()
                                .py_2()
                                .border_b_1()
                                .border_color(p.line)
                                .child(
                                    div()
                                        .w(px(40.0))
                                        .flex_none()
                                        .font_family(cx.theme().mono_font_family.clone())
                                        .text_size(px(12.0))
                                        .child(record.marker_index.to_string()),
                                )
                                .child(
                                    div().w(px(172.0)).flex_none().child(
                                        Button::new(("copy-kks", range.start + record_index))
                                            .ghost()
                                            .xsmall()
                                            .justify_start()
                                            .label(short_label(&kks, 22))
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .tooltip(format!("Копировать: {kks}"))
                                            .on_click(cx.listener(move |_, _, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    kks.clone(),
                                                ))
                                            })),
                                    ),
                                )
                                .child(
                                    div().flex_1().min_w(px(0.0)).text_size(px(12.0)).child(
                                        record
                                            .description
                                            .clone()
                                            .unwrap_or_else(|| "Нет описания в базе".into()),
                                    ),
                                ),
                        );
                    }
                }
            }
            table = table.child(rows);
        }
        table = table.child(
            div()
                .flex_none()
                .px_4()
                .py_3()
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .child(hint(format!("Страница {} из {pages}", self.page + 1), p))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("previous")
                                .small()
                                .label("Назад")
                                .disabled(self.page == 0)
                                .on_click(cx.listener(|app, _, _, cx| {
                                    app.page = app.page.saturating_sub(1);
                                    app.expanded_frame = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("next")
                                .small()
                                .label("Далее")
                                .disabled(self.page + 1 >= pages)
                                .on_click(cx.listener(|app, _, _, cx| {
                                    app.page += 1;
                                    app.expanded_frame = None;
                                    cx.notify();
                                })),
                        ),
                ),
        );
        div()
            .flex_1()
            .min_h(px(0.0))
            .px_6()
            .pb_6()
            .flex()
            .gap_5()
            .child(sidebar)
            .child(table)
            .into_any_element()
    }

    fn history_page(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.settings.dark);
        if self.jobs.is_empty() {
            return div().flex_1().px_6().pb_6().child(surface(p).h_full().flex().flex_col().items_center().justify_center()
                .child(empty(IconName::BookOpen, "Запусков пока нет", "Здесь сохраняются результаты, параметры и логи последних 30 обработок.", p))
                .child(Button::new("history-home").primary().label("Создать первый пакет")
                    .on_click(cx.listener(|app, _, window, cx| app.navigate(0, window, cx))))).into_any_element();
        }
        let selected = self.selected_job.unwrap_or(self.jobs.len() - 1);
        let mut list = surface(p)
            .id("history-list")
            .w(px(258.0))
            .flex_none()
            .overflow_y_scroll()
            .p_2()
            .flex()
            .flex_col()
            .gap_2();
        for (i, job) in self.jobs.iter().enumerate().rev() {
            let color = if self.running == Some(i) {
                p.accent
            } else if job.error.is_some() || job.failed > 0 {
                p.danger
            } else {
                p.good
            };
            list = list.child(
                div()
                    .id(("history-entry", i))
                    .p_3()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .border_1()
                    .border_color(if i == selected { p.accent } else { p.line })
                    .bg(if i == selected { p.selected } else { p.surface })
                    .hover(|v| v.bg(p.inset))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(job.label.clone()),
                            )
                            .child(
                                Icon::new(if job.error.is_some() || job.failed > 0 {
                                    IconName::CircleX
                                } else {
                                    IconName::CircleCheck
                                })
                                .small()
                                .text_color(color),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(12.0))
                            .text_color(color)
                            .child(format!("{} · {} DOCX", job.status, count(job.success))),
                    )
                    .child(
                        hint(
                            format!("{} · {} кадров", duration(job.elapsed()), count(job.total)),
                            p,
                        )
                        .mt_1(),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.selected_job = Some(i);
                        app.show_logs = false;
                        cx.notify();
                    })),
            );
        }
        div()
            .flex_1()
            .min_h(px(0.0))
            .px_6()
            .pb_6()
            .flex()
            .gap_5()
            .child(list)
            .child(
                div()
                    .id("history-detail")
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_y_scroll()
                    .child(self.job_panel(selected, true, cx)),
            )
            .into_any_element()
    }
}
