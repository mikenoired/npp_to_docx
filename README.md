# npp_to_docx

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
bun run src/index.ts
```

В терминале отображается однострочный progress bar, а детальные логи по файлам пишутся в лог-файл формата `<дата-время>.log` в корне проекта.

## Полезные опции

```bash
bun run src/index.ts --limit 5
bun run src/index.ts --match TAB --concurrency 2
bun run src/index.ts --input input --output output
```
