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
5. Следите за `Active job` и историей запусков

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

## Полезные опции

```bash
bun run start:cli --limit 5
bun run start:cli --match TAB --concurrency 2
bun run start:cli --input input --output output
bun run build:bins
```
