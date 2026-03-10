import { LayoutDashboard, Moon, Search, Sun } from "lucide-react";
import { useEffect, useEffectEvent, useState } from "react";

import type {
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
import { Button } from "./components/ui/button";
import { TabsList, TabsTrigger } from "./components/ui/tabs";

type AppTab = "home" | "search";
type ThemeMode = "light" | "dark";
const THEME_STORAGE_KEY = "npp_to_docx.theme";

function getDesktopApi() {
  const desktopApi = window.nppApi;
  if (!desktopApi) {
    throw new Error("Desktop API недоступен. Приложение запущено без preload Electron.");
  }
  return desktopApi;
}

export default function App() {
  const [theme, setTheme] = useState<ThemeMode>(() => {
    if (typeof window === "undefined") {
      return "light";
    }
    return window.localStorage.getItem(THEME_STORAGE_KEY) === "dark" ? "dark" : "light";
  });
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
  const [errorMessage, setErrorMessage] = useState<string>();
  const [workspaceDir, setWorkspaceDir] = useState("");
  const [logsDir, setLogsDir] = useState("");
  const [searchStatus, setSearchStatus] = useState<SearchStatusResponse>();
  const [submodelQuery, setSubmodelQuery] = useState("");
  const [submodelOptions, setSubmodelOptions] = useState<SearchSubmodelOption[]>([]);
  const [searchResults, setSearchResults] = useState<SearchResultItem[]>([]);
  const [searchPage, setSearchPage] = useState(1);
  const activeJob = jobs.find((job) => job.id === activeJobId) ?? jobs[0];

  const loadSearchStatusFor = useEffectEvent(async (nextInputDir: string, nextOutputDir: string) => {
    const data = (await getDesktopApi().getSearchStatus({
      inputDir: nextInputDir,
      outputDir: nextOutputDir,
    })) as SearchStatusResponse;
    setSearchStatus(data);
  });

  const loadJobs = useEffectEvent(async () => {
    const data = (await getDesktopApi().listJobs()) as JobsResponse;
    setJobs(data.jobs);
  });

  const loadSearchStatus = useEffectEvent(async () => {
    await loadSearchStatusFor(inputDir, outputDir);
  });

  async function prepareDb() {
    setBusy(true);
    setErrorMessage(undefined);
    try {
      await getDesktopApi().prepareDb({ inputDir });
      await loadSearchStatus();
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function runJob() {
    setBusy(true);
    setErrorMessage(undefined);
    try {
      const data = (await getDesktopApi().createJob({
        inputDir,
        outputDir,
        concurrency: Number.parseInt(concurrency, 10) || 1,
        match: match || undefined,
        limit: limit ? Number.parseInt(limit, 10) : undefined,
      })) as CreateJobResponse;
      setActiveJobId(data.id);
      await loadJobs();
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function pickDirectory(kind: "input" | "output") {
    const selected = await getDesktopApi().selectDirectory({
      title: kind === "input" ? "Выберите входную директорию" : "Выберите выходную директорию",
      defaultPath: kind === "input" ? inputDir : outputDir,
    });

    if (!selected) {
      return;
    }

    if (kind === "input") {
      setInputDir(selected);
      return;
    }

    setOutputDir(selected);
  }

  async function runSearch() {
    if (!submodelQuery.trim()) {
      return;
    }

    setSearchBusy(true);
    setErrorMessage(undefined);
    try {
      const data = (await getDesktopApi().searchBySubmodel({
        outputDir,
        submodel: submodelQuery.trim(),
      })) as SearchResultsResponse;
      setSearchResults(data.items);
      setSearchPage(1);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setSearchBusy(false);
    }
  }

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    void (async () => {
      try {
        const config = await getDesktopApi().getConfig();
        setWorkspaceDir(config.defaults.workspaceDir);
        setInputDir(config.defaults.inputDir);
        setOutputDir(config.defaults.outputDir);
        setLogsDir(config.defaults.logsDir);
        setConcurrency(String(config.defaults.concurrency));
        await loadJobs();
        await loadSearchStatusFor(config.defaults.inputDir, config.defaults.outputDir);
      } catch (error) {
        setErrorMessage(error instanceof Error ? error.message : String(error));
      }
    })();
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      void (async () => {
        try {
          await loadJobs();
          await loadSearchStatus();
        } catch (error) {
          setErrorMessage(error instanceof Error ? error.message : String(error));
        }
      })();
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
        try {
          const data = (await getDesktopApi().listSearchSubmodels({
            outputDir,
            query: submodelQuery.trim(),
          })) as SearchSubmodelsResponse;
          setSubmodelOptions(data.items);
        } catch (error) {
          setErrorMessage(error instanceof Error ? error.message : String(error));
        }
      })();
    }, 150);
    return () => window.clearTimeout(timeoutId);
  }, [submodelQuery, outputDir, searchStatus?.ready]);

  return (
    <main className="mx-auto flex min-h-screen max-w-7xl flex-col gap-6 px-4 py-8 text-[color:var(--text)] md:px-8">
      <section className="rounded-2xl border border-[color:var(--border)] bg-[var(--panel)] px-5 py-4 shadow-[var(--shadow)] backdrop-blur">
        <div className="flex flex-col gap-4 text-sm text-[color:var(--text-muted)] md:flex-row md:items-center md:justify-between">
          <div>
            <div className="text-base font-semibold text-[color:var(--text)]">NPP Processor</div>
            <div>Workspace: {workspaceDir || "Определяем..."}</div>
          </div>
          <div className="flex flex-col gap-3 md:items-end">
            <div>Логи: {logsDir || "Определяем..."}</div>
            <div>Движок обработки встроен в приложение и работает напрямую с файловой системой.</div>
            <Button variant="outline" onClick={() => setTheme(theme === "light" ? "dark" : "light")}>
              {theme === "light" ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
              {theme === "light" ? "Темная тема" : "Светлая тема"}
            </Button>
          </div>
        </div>
        {errorMessage ? (
          <div className="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
            {errorMessage}
          </div>
        ) : null}
      </section>

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
          onSelectInputDir={() => void pickDirectory("input")}
          onSelectOutputDir={() => void pickDirectory("output")}
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
