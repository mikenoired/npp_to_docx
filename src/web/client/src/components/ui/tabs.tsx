import type * as React from "react";

import { cn } from "../../lib/utils";

function TabsList({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn("inline-flex items-center rounded-xl border border-slate-200 bg-white p-1 gap-2 shadow-sm", className)}
      {...props}
    />
  );
}

function TabsTrigger({ className, active = false, ...props }: React.ComponentProps<"button"> & { active?: boolean }) {
  return (
    <button
      className={cn(
        "inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors cursor-pointer",
        active ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100 hover:text-slate-950",
        className,
      )}
      type="button"
      {...props}
    />
  );
}

export { TabsList, TabsTrigger };
