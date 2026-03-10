import type { JobDto } from "../../../../../shared/api";
import { ActiveJobCard } from "./active-job-card";
import { ControlPanelCard } from "./control-panel-card";
import { JobsHistoryCard } from "./jobs-history-card";

type HomeTabProps = {
  busy: boolean;
  inputDir: string;
  outputDir: string;
  concurrency: string;
  match: string;
  limit: string;
  jobs: JobDto[];
  activeJob?: JobDto;
  onInputDirChange(value: string): void;
  onOutputDirChange(value: string): void;
  onConcurrencyChange(value: string): void;
  onMatchChange(value: string): void;
  onLimitChange(value: string): void;
  onPrepareDb(): void;
  onRunJob(): void;
  onSelectJob(id: string): void;
};

export function HomeTab(props: HomeTabProps) {
  return (
    <>
      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <ControlPanelCard
          busy={props.busy}
          inputDir={props.inputDir}
          outputDir={props.outputDir}
          concurrency={props.concurrency}
          match={props.match}
          limit={props.limit}
          onInputDirChange={props.onInputDirChange}
          onOutputDirChange={props.onOutputDirChange}
          onConcurrencyChange={props.onConcurrencyChange}
          onMatchChange={props.onMatchChange}
          onLimitChange={props.onLimitChange}
          onPrepareDb={props.onPrepareDb}
          onRunJob={props.onRunJob}
        />
        <ActiveJobCard activeJob={props.activeJob} />
      </section>

      <JobsHistoryCard jobs={props.jobs} onSelectJob={props.onSelectJob} />
    </>
  );
}
