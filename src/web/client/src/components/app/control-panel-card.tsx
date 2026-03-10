import { LoaderCircle, Play, RefreshCcw, TableProperties } from "lucide-react";

import { Button } from "../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Input } from "../ui/input";

type ControlPanelCardProps = {
  busy: boolean;
  inputDir: string;
  outputDir: string;
  concurrency: string;
  match: string;
  limit: string;
  onInputDirChange(value: string): void;
  onOutputDirChange(value: string): void;
  onConcurrencyChange(value: string): void;
  onMatchChange(value: string): void;
  onLimitChange(value: string): void;
  onPrepareDb(): void;
  onRunJob(): void;
};

export function ControlPanelCard(props: ControlPanelCardProps) {
  return (
    <Card className="overflow-hidden">
      <CardHeader className="border-b border-slate-200 bg-white/70 backdrop-blur">
        <CardTitle className="flex items-center gap-3">
          <span className="inline-flex h-10 w-10 items-center justify-center rounded-xl bg-slate-900 text-white">
            <TableProperties className="h-5 w-5" />
          </span>
          npp_to_docx
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4 pt-6 md:grid-cols-2">
        <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="input-dir">
          Директория на входе
          <Input
            id="input-dir"
            value={props.inputDir}
            onChange={(event) => props.onInputDirChange(event.target.value)}
          />
        </label>
        <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="output-dir">
          Директория на выходе
          <Input
            id="output-dir"
            value={props.outputDir}
            onChange={(event) => props.onOutputDirChange(event.target.value)}
          />
        </label>
        <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="concurrency">
          Обработок одновременно
          <Input
            id="concurrency"
            value={props.concurrency}
            onChange={(event) => props.onConcurrencyChange(event.target.value)}
          />
        </label>
        <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="match">
          Совпадение по названию файла
          <Input
            id="match"
            value={props.match}
            onChange={(event) => props.onMatchChange(event.target.value)}
            placeholder="например 4UJ"
          />
        </label>
        <label className="grid gap-2 text-sm font-medium text-slate-700 md:col-span-2" htmlFor="limit">
          Общий лимит обработок
          <Input
            id="limit"
            value={props.limit}
            onChange={(event) => props.onLimitChange(event.target.value)}
            placeholder="например 5"
          />
        </label>
        <div className="flex flex-wrap gap-3 md:col-span-2">
          <Button variant="secondary" disabled={props.busy} onClick={props.onPrepareDb}>
            {props.busy ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <RefreshCcw className="h-4 w-4" />}
            Подготовить базу данных
          </Button>
          <Button disabled={props.busy} onClick={props.onRunJob}>
            {props.busy ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
            Запустить обработку
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
