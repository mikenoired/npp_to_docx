import { LoaderCircle, Play, RefreshCcw, TableProperties } from "lucide-react";
import { useEffect, useEffectEvent, useState } from "react";

import type { AppConfigResponse, CreateJobResponse, JobDto, JobsResponse } from "../../../shared/api";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { Input } from "./components/ui/input";

function formatProgressLabel(job: JobDto): string {
  const total = job.progress?.total ?? job.progress?.completed ?? job.result?.success ?? 0;
  const completed = job.progress?.completed ?? total;
  return total > 0 ? `${completed}/${total}` : job.status;
}

function getProgressSegments(job: JobDto): { success: number; failed: number; pending: number } {
  const total = job.progress?.total ?? job.progress?.completed ?? job.result?.success ?? job.result?.failed ?? 0;
  if (total <= 0) {
    return {
      success: 0,
      failed: 0,
      pending: job.status === "running" || job.status === "queued" ? 100 : 0,
    };
  }

  const successCount = job.progress?.success ?? job.result?.success ?? 0;
  const failedCount = job.progress?.failed ?? job.result?.failed ?? 0;
  const completedCount = job.progress?.completed ?? successCount + failedCount;
  const pendingCount = Math.max(0, total - completedCount);

  return {
    success: (successCount / total) * 100,
    failed: (failedCount / total) * 100,
    pending: (pendingCount / total) * 100,
  };
}

function JobProgressBar({ job }: { job: JobDto }) {
  const segments = getProgressSegments(job);

  return (
    <div className="min-w-[220px] space-y-2">
      <div className="h-3 overflow-hidden rounded-full bg-slate-200">
        <div className="flex h-full w-full">
          <div className="bg-emerald-500" style={{ width: `${segments.success}%` }} />
          <div className="bg-rose-500" style={{ width: `${segments.failed}%` }} />
          <div className="bg-slate-400" style={{ width: `${segments.pending}%` }} />
        </div>
      </div>
      <div className="flex items-center justify-between text-xs text-slate-600">
        <span>{formatProgressLabel(job)}</span>
        <span>{job.status}</span>
      </div>
    </div>
  );
}

export default function App() {
  const [inputDir, setInputDir] = useState("input");
  const [outputDir, setOutputDir] = useState("output");
  const [concurrency, setConcurrency] = useState("2");
  const [match, setMatch] = useState("");
  const [limit, setLimit] = useState("");
  const [jobs, setJobs] = useState<JobDto[]>([]);
  const [activeJobId, setActiveJobId] = useState<string>();
  const [busy, setBusy] = useState(false);
  const activeJob = jobs.find((job) => job.id === activeJobId) ?? jobs[0];

  const loadConfig = useEffectEvent(async () => {
    const response = await fetch("/api/config");
    const data = (await response.json()) as AppConfigResponse;
    setInputDir(data.defaults.inputDir);
    setOutputDir(data.defaults.outputDir);
    setConcurrency(String(data.defaults.concurrency));
  });

  const loadJobs = useEffectEvent(async () => {
    const response = await fetch("/api/jobs");
    const data = (await response.json()) as JobsResponse;
    setJobs(data.jobs);
  });

  async function prepareDb() {
    setBusy(true);
    try {
      await fetch("/api/db/prepare", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ inputDir }),
      });
    } finally {
      setBusy(false);
    }
  }

  async function runJob() {
    setBusy(true);
    try {
      const response = await fetch("/api/jobs", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          inputDir,
          outputDir,
          concurrency: Number.parseInt(concurrency, 10) || 1,
          match: match || undefined,
          limit: limit ? Number.parseInt(limit, 10) : undefined,
        }),
      });
      const data = (await response.json()) as CreateJobResponse;
      setActiveJobId(data.id);
      await loadJobs();
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void loadConfig();
    void loadJobs();
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      void loadJobs();
    }, 1000);
    return () => window.clearInterval(interval);
  }, []);

  return (
    <main className="mx-auto flex min-h-screen max-w-7xl flex-col gap-6 px-4 py-8 md:px-8">
      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <Card className="overflow-hidden">
          <CardHeader className="border-b border-slate-200 bg-white/70 backdrop-blur">
            <CardTitle className="flex items-center gap-3">
              <span className="inline-flex h-10 w-10 items-center justify-center rounded-xl bg-slate-900 text-white">
                <TableProperties className="h-5 w-5" />
              </span>
              npp_to_docx Control Panel
            </CardTitle>
            <CardDescription>
              Один и тот же движок обработки для CLI и web. Здесь мы управляем подготовкой CSV и запуском обработки SVG.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 pt-6 md:grid-cols-2">
            <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="input-dir">
              Input directory
              <Input id="input-dir" value={inputDir} onChange={(event) => setInputDir(event.target.value)} />
            </label>
            <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="output-dir">
              Output directory
              <Input id="output-dir" value={outputDir} onChange={(event) => setOutputDir(event.target.value)} />
            </label>
            <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="concurrency">
              Concurrency
              <Input id="concurrency" value={concurrency} onChange={(event) => setConcurrency(event.target.value)} />
            </label>
            <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="match">
              Match
              <Input
                id="match"
                value={match}
                onChange={(event) => setMatch(event.target.value)}
                placeholder="например 4UJ"
              />
            </label>
            <label className="grid gap-2 text-sm font-medium text-slate-700 md:col-span-2" htmlFor="limit">
              Limit
              <Input
                id="limit"
                value={limit}
                onChange={(event) => setLimit(event.target.value)}
                placeholder="например 5"
              />
            </label>
            <div className="flex flex-wrap gap-3 md:col-span-2">
              <Button variant="secondary" disabled={busy} onClick={() => void prepareDb()}>
                {busy ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <RefreshCcw className="h-4 w-4" />}
                Подготовить CSV
              </Button>
              <Button disabled={busy} onClick={() => void runJob()}>
                {busy ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                Запустить обработку
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Активный job</CardTitle>
            <CardDescription>
              Polling раз в секунду. На первом этапе этого достаточно, SSE/WebSocket не нужен.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {!activeJob ? (
              <div className="rounded-lg border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500">
                Запусков пока нет.
              </div>
            ) : (
              <>
                <div className="grid gap-2 text-sm text-slate-700">
                  <div>
                    <span className="font-semibold">ID:</span> {activeJob.id}
                  </div>
                  <div>
                    <span className="font-semibold">Status:</span> {activeJob.status}
                  </div>
                  {activeJob.progress ? (
                    <div>
                      <span className="font-semibold">Progress:</span> {activeJob.progress.completed}/
                      {activeJob.progress.total}
                    </div>
                  ) : null}
                  {activeJob.result ? (
                    <div className="rounded-lg bg-slate-950 px-3 py-2 text-slate-50">{activeJob.result.summary}</div>
                  ) : null}
                  {activeJob.error ? (
                    <div className="rounded-lg bg-rose-100 px-3 py-2 text-rose-700">{activeJob.error}</div>
                  ) : null}
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </section>

      <Card>
        <CardHeader>
          <CardTitle>История jobs</CardTitle>
          <CardDescription>Последние запуски web-обработчика.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="overflow-hidden rounded-xl border border-slate-200">
            <table className="w-full border-collapse text-sm">
              <thead className="bg-slate-100 text-left text-slate-700">
                <tr>
                  <th className="px-4 py-3">Job</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Input</th>
                  <th className="px-4 py-3">Output</th>
                  <th className="px-4 py-3">Progress</th>
                </tr>
              </thead>
              <tbody>
                {jobs.map((job) => (
                  <tr
                    key={job.id}
                    className="cursor-pointer border-t border-slate-200 bg-white hover:bg-slate-50"
                    onClick={() => setActiveJobId(job.id)}
                  >
                    <td className="px-4 py-3 font-mono text-xs">{job.id.slice(0, 8)}</td>
                    <td className="px-4 py-3">{job.status}</td>
                    <td className="px-4 py-3">{job.options.inputDir}</td>
                    <td className="px-4 py-3">{job.options.outputDir}</td>
                    <td className="px-4 py-3">
                      <JobProgressBar job={job} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </main>
  );
}
