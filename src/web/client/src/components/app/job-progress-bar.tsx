import type { JobDto } from "../../../../../shared/api";

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

export function JobProgressBar({ job }: { job: JobDto }) {
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
