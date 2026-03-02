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

1. Поместите в папку с программой папку `input`, в котором будут располагаться svg-файлы
2. Выполните установку зависимостей с помощью комманды `bun install`. Выполнить команду в папке с программой

## Запуск

```bash
bun run src/index.ts
```

## Полезные опции

```bash
bun run src/index.ts --limit 5
bun run src/index.ts --match TAB --concurrency 2
bun run src/index.ts --input input --output output
```
