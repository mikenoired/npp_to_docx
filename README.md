# npp_to_docx

## Архитектура

Проект теперь разделён на логические слои:

- `src/core` — общий движок обработки SVG/DB/DOCX
- `src/cli` — CLI entrypoint и парсинг аргументов
- `src/web/server` — локальный Bun HTTP server и API
- `src/web/client` — React 19 + Tailwind + shadcn-style интерфейс
- `src/shared` — общие типы API между сервером и фронтом

CLI и web используют один и тот же processing layer из `src/core`.

## Установка

Предварительно требуется установить Bun для установки зависимостей и запуска программы через терминал.

Для Windows:
```powershell
powershell -c "irm bun.sh/install.ps1 | iex"
```

Для macOS/Linux
```bash
curl -fsSL https://bun.sh/install | bash
```

1. Поместите в папку с программой папку `input`
2. В `input` должны лежать файлы `PLS_ANA_CONF.dmp`, `PLS_BIN_CONF.dmp` и подпапка `svg` с исходными схемами
3. При первом запуске рядом с `.dmp` будут автоматически созданы `PLS_ANA_CONF.csv` и `PLS_BIN_CONF.csv` в UTF-8
4. Выполните установку зависимостей с помощью комманды `bun install`. Выполнить команду в папке с программой

## Запуск

```bash
bun run start:cli
bun run start:web
```

В терминале отображается однострочный progress bar, а детальные логи по файлам пишутся в лог-файл формата `<дата-время>.log` в корне проекта.

Web-режим поднимает локальный HTTP server с React UI и API для запуска обработки. По умолчанию используется `http://127.0.0.1:3000`.

## Dev-режим

Одна команда для frontend и backend:

```bash
bun run dev:web
```

По умолчанию:

- frontend: `http://127.0.0.1:5173`
- backend API: `http://127.0.0.1:3300`

Если нужно сменить порты:

```bash
DEV_WEB_SERVER_PORT=3400 DEV_WEB_UI_PORT=5174 bun run dev:web
```

## Использование

### CLI

```bash
bun run start:cli --match 4UJ --limit 1
```

Что делает CLI:

- при необходимости создаёт `PLS_ANA_CONF.csv` и `PLS_BIN_CONF.csv` рядом с `.dmp`
- загружает CSV
- обрабатывает SVG из `input/svg`
- складывает готовые `.docx` в `output`

### Web

```bash
bun run start:web
```

Дальше:

1. Откройте браузер на `http://127.0.0.1:3000`
2. Проверьте `input` и `output`
3. Нажмите `Подготовить CSV`
4. Нажмите `Запустить обработку`
5. После успешной обработки используйте блок `Поиск по подмодели`
6. Выберите подмодель, например `DS_ana.svg`, и нажмите `Показать видеокадры`
7. Система покажет видеокадры, номер привязки, KKS и описание

## Сборка бинарников

```bash
bun run build:bins
```

Результат:

- `dist/npp-cli`
- `dist/npp-web`

Запуск бинарников:

```bash
./dist/npp-cli --match 4UJ --limit 1
./dist/npp-web
```

## Упаковка для запуска двойным кликом

```bash
bun run package:target
```

После этого появится папка `target`:

- `target/NppToDocx.app` — основной файл для запуска двойным кликом
- `target/input` — входные данные
- `target/output` — выходные файлы
- `target/logs` — лог web-сервера
- `target/npp-cli` — CLI бинарник для терминала

Для обычного пользователя достаточно открыть:

```text
target/NppToDocx.app
```

Приложение поднимет локальный сервер и откроет браузер автоматически.

## Полезные опции

```bash
bun run start:cli --limit 5
bun run start:cli --match TAB --concurrency 2
bun run start:cli --input input --output output
bun run build:bins
```
