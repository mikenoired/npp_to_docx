import type { JobDto } from "../../../../../shared/api";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { JobProgressBar } from "./job-progress-bar";

type JobsHistoryCardProps = {
  jobs: JobDto[];
  onSelectJob(id: string): void;
};

export function JobsHistoryCard({ jobs, onSelectJob }: JobsHistoryCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>История обработок</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-xl border border-slate-200">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-slate-100 text-left text-slate-700">
              <tr>
                <th className="px-4 py-3">Код процесса</th>
                <th className="px-4 py-3">Статус</th>
                <th className="px-4 py-3">Входная папка</th>
                <th className="px-4 py-3">Выходная папка</th>
                <th className="px-4 py-3">Прогресс</th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr
                  key={job.id}
                  className="cursor-pointer border-t border-slate-200 bg-white hover:bg-slate-50"
                  onClick={() => onSelectJob(job.id)}
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
  );
}
