import type { JobDto } from "../../../../../shared/api";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";

export function ActiveJobCard({ activeJob }: { activeJob?: JobDto }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Последняя обработка</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {!activeJob ? (
          <div className="rounded-lg border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500">
            Запусков пока нет.
          </div>
        ) : (
          <div className="space-y-4">
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
            <div className="max-h-72 overflow-auto rounded-xl bg-slate-950">
              <pre className="whitespace-pre max-w-80 p-4 text-xs leading-6 text-slate-100">
                {(activeJob.logs ?? []).join("\n") || activeJob.progress?.line || "Логи появятся после старта"}
              </pre>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
