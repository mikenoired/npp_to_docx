import { LayoutDashboard, Search } from "lucide-react";
import { useEffect, useEffectEvent, useState } from "react";

import type {
  AppConfigResponse,
  CreateJobResponse,
  JobDto,
  JobsResponse,
  SearchResultItem,
  SearchResultsResponse,
  SearchStatusResponse,
  SearchSubmodelOption,
  SearchSubmodelsResponse,
} from "../../../shared/api";
import { HomeTab } from "./components/app/home-tab";
import { SearchSubmodelCard } from "./components/app/search-submodel-card";
import { TabsList, TabsTrigger } from "./components/ui/tabs";

type AppTab = "home" | "search";

export default function App() {
  const [activeTab, setActiveTab] = useState<AppTab>("home");
  const [inputDir, setInputDir] = useState("input");
  const [outputDir, setOutputDir] = useState("output");
  const [concurrency, setConcurrency] = useState("2");
  const [match, setMatch] = useState("");
  const [limit, setLimit] = useState("");
  const [jobs, setJobs] = useState<JobDto[]>([]);
  const [activeJobId, setActiveJobId] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [searchBusy, setSearchBusy] = useState(false);
  const [searchStatus, setSearchStatus] = useState<SearchStatusResponse>();
  const [submodelQuery, setSubmodelQuery] = useState("");
  const [submodelOptions, setSubmodelOptions] = useState<SearchSubmodelOption[]>([]);
  const [searchResults, setSearchResults] = useState<SearchResultItem[]>([]);
  const [searchPage, setSearchPage] = useState(1);
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

  const loadSearchStatus = useEffectEvent(async () => {
    const params = new URLSearchParams({
      inputDir,
      outputDir,
    });
    const response = await fetch(`/api/search/status?${params.toString()}`);
    const data = (await response.json()) as SearchStatusResponse;
    setSearchStatus(data);
  });

  async function prepareDb() {
    setBusy(true);
    try {
      await fetch("/api/db/prepare", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ inputDir }),
      });
      await loadSearchStatus();
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

  async function runSearch() {
    if (!submodelQuery.trim()) {
      return;
    }

    setSearchBusy(true);
    try {
      const params = new URLSearchParams({
        outputDir,
        submodel: submodelQuery.trim(),
      });
      const response = await fetch(`/api/search/results?${params.toString()}`);
      const data = (await response.json()) as SearchResultsResponse;
      setSearchResults(data.items);
      setSearchPage(1);
    } finally {
      setSearchBusy(false);
    }
  }

  useEffect(() => {
    void loadConfig();
    void loadJobs();
    void loadSearchStatus();
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      void loadJobs();
      void loadSearchStatus();
    }, 1000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    const timeoutId = window.setTimeout(() => {
      if (!searchStatus?.ready) {
        setSubmodelOptions([]);
        return;
      }

      void (async () => {
        const params = new URLSearchParams({
          outputDir,
          q: submodelQuery.trim(),
        });
        const response = await fetch(`/api/search/submodels?${params.toString()}`);
        const data = (await response.json()) as SearchSubmodelsResponse;
        setSubmodelOptions(data.items);
      })();
    }, 150);
    return () => window.clearTimeout(timeoutId);
  }, [submodelQuery, outputDir, searchStatus?.ready]);

  return (
    <main className="mx-auto flex min-h-screen max-w-7xl flex-col gap-6 px-4 py-8 md:px-8">
      <TabsList>
        <TabsTrigger active={activeTab === "home"} onClick={() => setActiveTab("home")}>
          <LayoutDashboard className="h-4 w-4" />
          Главная
        </TabsTrigger>
        <TabsTrigger active={activeTab === "search"} onClick={() => setActiveTab("search")}>
          <Search className="h-4 w-4" />
          Поиск
        </TabsTrigger>
      </TabsList>

      {activeTab === "home" ? (
        <HomeTab
          busy={busy}
          inputDir={inputDir}
          outputDir={outputDir}
          concurrency={concurrency}
          match={match}
          limit={limit}
          jobs={jobs}
          activeJob={activeJob}
          onInputDirChange={setInputDir}
          onOutputDirChange={setOutputDir}
          onConcurrencyChange={setConcurrency}
          onMatchChange={setMatch}
          onLimitChange={setLimit}
          onPrepareDb={() => void prepareDb()}
          onRunJob={() => void runJob()}
          onSelectJob={setActiveJobId}
        />
      ) : (
        <SearchSubmodelCard
          busy={searchBusy}
          searchStatus={searchStatus}
          submodelQuery={submodelQuery}
          submodelOptions={submodelOptions}
          searchResults={searchResults}
          searchPage={searchPage}
          onSubmodelQueryChange={(value) => {
            setSubmodelQuery(value);
            setSearchPage(1);
          }}
          onSearch={() => void runSearch()}
          onSearchPageChange={setSearchPage}
        />
      )}
    </main>
  );
}
