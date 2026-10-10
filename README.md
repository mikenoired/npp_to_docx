# npp_to_docx

Нативное desktop-приложение на Rust и GPUI: преобразование SVG-кадров в паспорта
DOCX с нумерованной схемой, таблицей KKS/описаний/подмоделей и поиском по подмодели.
Приложение и CLI используют один движок. Готовая сборка не требует Node, Bun или Electron.

## Запуск

Распакуйте архив для своей ОС и запустите `npp-to-docx` (Windows: `.exe`, macOS:
`NppToDocx.app`). Укажите входную и выходную папки. Во входной нужны
`PLS_ANA_CONF.dmp` и `PLS_BIN_CONF.dmp` либо подготовленные одноимённые CSV.
SVG читаются из `input/svg`, если такая подпапка существует, иначе из входной
папки. Можно выбрать отдельную папку SVG.

«Подготовить базу данных» создаёт CSV из DMP в Windows-1251.
«Запустить обработку» создаёт DOCX, `passport-report.json` и `search-index.json`.
Ошибки отдельных кадров не прерывают остальные. Отсутствующие подмодели показаны
прямоугольником с крестом и перечислены в документе. Сетевые изображения не загружаются.

Настройки папок и темы сохраняются в системной папке конфигурации. Рабочая папка
по умолчанию — `Documents/npp_to_docx`; логи запусков находятся в её `logs`.
История запусков хранится в памяти текущего сеанса.

## CLI

```sh
npp-to-docx prepare --input input
npp-to-docx convert --input input --output output --concurrency 2
npp-to-docx convert --input input --svg frames --output output --match 4UJ --limit 10
npp-to-docx search --output output --submodel DS_ana.svg
```

`convert --result-json result.json` сохраняет счётчики и время обработки.
Ошибка запуска или неуспешные кадры дают ненулевой код завершения. `inspect --svg`
выводит маркеры и координаты. Отдельный `npp-convert` содержит те же команды
без зависимости от GUI и подходит для серверной обработки.

## Сборка и проверки

Нужен современный stable Rust с edition 2024. Версии зафиксированы в `Cargo.lock`.

```sh
cargo build --release --locked --workspace
cargo run --release -p npp-to-docx
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/package.py
```

macOS: нужны Xcode Command Line Tools. GPUI использует Metal; шейдеры компилируются
при запуске штатным механизмом `runtime_shaders`.

Linux: нужны GPU, X11 или Wayland и системные библиотеки. На Ubuntu 24.04 для сборки:

```sh
sudo apt-get install clang libclang-dev pkg-config libasound2-dev libfontconfig1-dev \
  libfreetype6-dev libssl-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libx11-xcb-dev fonts-liberation
```

Windows: Rust MSVC и Visual Studio Build Tools с Desktop development with C++.
Для bindgen нужен LLVM/Clang и `LIBCLANG_PATH`, указывающий на его `bin`.
GPUI использует DirectX. Шрифты для схем берутся из ОС; при отсутствии шрифта SVG
применяется системная замена.

CI `.github/workflows/rust.yml` собирает и проверяет проект на трёх ОС и публикует
macOS `.app` в ZIP, Linux `tar.gz`, Windows `.exe` в ZIP. Linux-архив использует
системные библиотеки и не является универсальным статическим бинарником.

## Проверка миграции и замеры

Исходная версия Electron сохранена в Git: `fef9860c816f796c26442c68218461cf375eaa38`.
`input`, `output`, `target`, `dist` и `node_modules` игнорируются. Эталонные документы:
`output/electron-reference`; результат Rust: `output/rust-reference`.

```sh
python3 scripts/prepare_electron_baseline.py
node scripts/electron_run.mjs output/electron-baseline input output/electron-reference output/electron-result.json
npp-to-docx convert --input input --output output/rust-reference --concurrency 2
python3 scripts/compare_outputs.py output/electron-reference output/rust-reference --report output/parity.json
```

`compare_outputs.py --raster` дополнительно измеряет различия пикселей (нужен Pillow).
Сравниваются все тексты DOCX, таблицы, цвета несовпадений, размеры страницы и рисунков,
отчёт ресурсов и записи поиска. resvg и librsvg имеют разное сглаживание; изображения
оцениваются отдельно от содержимого документов.

Для обеих реализаций `scripts/benchmark.py` сохраняет одинаковую схему JSON:
команду, среду, хеш входного набора, каждый прогон, медиану времени и пиковую RSS.
Используются подготовленные CSV, одинаковый набор и `concurrency=2`. Установка
зависимостей исключена из времени сборки. На Windows RSS записывается как `null`;
на macOS/Linux используется `/usr/bin/time`.

```sh
python3 scripts/benchmark.py --implementation rust --metric conversion_prepared_csv \
  --input input --runs 3 --output output/rust-benchmark.json -- \
  target/release/npp-to-docx convert --input input --output output/rust-reference --concurrency 2
```

Замеры и итоговое сравнение хранятся в `benchmarks/`; SVG и JPEG графика — там же.
Числа относятся к конкретной машине и данному набору файлов.
