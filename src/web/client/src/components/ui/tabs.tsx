import type * as React from "react";

import { cn } from "../../lib/utils";

function TabsList({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "inline-flex items-center gap-2 rounded-xl border border-[color:var(--border)] bg-[var(--panel-solid)] p-1 shadow-[var(--shadow)]",
        className,
      )}
      {...props}
    />
  );
}

function TabsTrigger({ className, active = false, ...props }: React.ComponentProps<"button"> & { active?: boolean }) {
  return (
    <button
      className={cn(
        "inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors cursor-pointer",
        active
          ? "bg-[var(--accent)] text-[var(--accent-text)]"
          : "text-[color:var(--text-muted)] hover:bg-[var(--panel-subtle)] hover:text-[color:var(--text)]",
        className,
      )}
      type="button"
      {...props}
    />
  );
}

export { TabsList, TabsTrigger };
