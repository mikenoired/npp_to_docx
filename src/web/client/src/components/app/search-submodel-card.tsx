import { LoaderCircle, TableProperties } from "lucide-react";

import type { SearchResultItem, SearchStatusResponse, SearchSubmodelOption } from "../../../../../shared/api";
import { Button } from "../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";
import { Input } from "../ui/input";

const SEARCH_RESULTS_PAGE_SIZE = 20;

type SearchSubmodelCardProps = {
  busy: boolean;
  searchStatus?: SearchStatusResponse;
  submodelQuery: string;
  submodelOptions: SearchSubmodelOption[];
  searchResults: SearchResultItem[];
  searchPage: number;
  onSubmodelQueryChange(value: string): void;
  onSearch(): void;
  onSearchPageChange(page: number): void;
};

export function SearchSubmodelCard({
  busy,
  searchStatus,
  submodelQuery,
  submodelOptions,
  searchResults,
  searchPage,
  onSubmodelQueryChange,
  onSearch,
  onSearchPageChange,
}: SearchSubmodelCardProps) {
  const searchTotalPages = Math.max(1, Math.ceil(searchResults.length / SEARCH_RESULTS_PAGE_SIZE));
  const pagedSearchResults = searchResults.slice(
    (searchPage - 1) * SEARCH_RESULTS_PAGE_SIZE,
    searchPage * SEARCH_RESULTS_PAGE_SIZE,
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle>Поиск по подмодели</CardTitle>
        <CardDescription>
          Показывает все видеокадры, где встречается выбранная подмодель, и номер привязки внутри кадра.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
          <div className="font-semibold text-slate-900">{searchStatus?.message ?? "Проверяем индекс поиска..."}</div>
          {searchStatus?.ready ? (
            <div className="mt-1 text-slate-600">
              Индекс найден: записей {searchStatus.recordsCount}, подмоделей {searchStatus.submodelsCount}.
            </div>
          ) : searchStatus?.missingFiles.length ? (
            <div className="mt-2 text-slate-600">Не хватает файлов: {searchStatus.missingFiles.join(", ")}</div>
          ) : null}
        </div>

        <div className="grid gap-4 md:grid-cols-[1fr_auto]">
          <label className="grid gap-2 text-sm font-medium text-slate-700" htmlFor="submodel-search">
            Подмодель
            <Input
              id="submodel-search"
              list="submodel-options"
              value={submodelQuery}
              disabled={!searchStatus?.ready}
              onChange={(event) => onSubmodelQueryChange(event.target.value)}
              placeholder="например DS_ana.svg"
            />
            <datalist id="submodel-options">
              {submodelOptions.map((item) => (
                <option key={item.submodel} value={item.submodel}>
                  {item.submodel} ({item.count})
                </option>
              ))}
            </datalist>
          </label>
          <div className="flex items-end">
            <Button disabled={!searchStatus?.ready || !submodelQuery.trim() || busy} onClick={onSearch}>
              {busy ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <TableProperties className="h-4 w-4" />}
              Показать видеокадры
            </Button>
          </div>
        </div>

        <div className="overflow-hidden rounded-xl border border-slate-200">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-slate-100 text-left text-slate-700">
              <tr>
                <th className="px-4 py-3">Видеокадр</th>
                <th className="px-4 py-3">№ привязки</th>
                <th className="px-4 py-3">KKS</th>
                <th className="px-4 py-3">Описание</th>
              </tr>
            </thead>
            <tbody>
              {pagedSearchResults.length > 0 ? (
                pagedSearchResults.map((item) => (
                  <tr key={`${item.frameName}-${item.markerIndex}`} className="border-t border-slate-200 bg-white">
                    <td className="px-4 py-3 font-medium text-slate-900">{item.frameName}</td>
                    <td className="px-4 py-3">{item.markerIndex}</td>
                    <td className="px-4 py-3">{item.kks ?? item.title}</td>
                    <td className="px-4 py-3">{item.description ?? "—"}</td>
                  </tr>
                ))
              ) : (
                <tr className="border-t border-slate-200 bg-white">
                  <td className="px-4 py-6 text-slate-500" colSpan={4}>
                    {searchStatus?.ready
                      ? "Выберите подмодель и запустите поиск."
                      : "Поиск станет доступен после успешной обработки и построения индекса."}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {searchResults.length > SEARCH_RESULTS_PAGE_SIZE ? (
          <div className="flex flex-col gap-3 text-sm text-slate-600 md:flex-row md:items-center md:justify-between">
            <div>
              Показаны {pagedSearchResults.length} из {searchResults.length}. Страница {searchPage} из{" "}
              {searchTotalPages}.
            </div>
            <div className="flex gap-2">
              <Button
                variant="secondary"
                disabled={searchPage <= 1}
                onClick={() => onSearchPageChange(Math.max(1, searchPage - 1))}
              >
                Назад
              </Button>
              <Button
                variant="secondary"
                disabled={searchPage >= searchTotalPages}
                onClick={() => onSearchPageChange(Math.min(searchTotalPages, searchPage + 1))}
              >
                Вперёд
              </Button>
            </div>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
