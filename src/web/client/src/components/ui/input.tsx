import * as React from "react";

import { cn } from "../../lib/utils";

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(({ className, ...props }, ref) => {
  return (
    <input
      className={cn(
        "flex h-10 w-full rounded-md border border-[color:var(--border-strong)] bg-[var(--input-bg)] px-3 py-2 text-sm text-[color:var(--text)] shadow-sm outline-none transition focus:border-[color:var(--accent)]",
        className,
      )}
      ref={ref}
      {...props}
    />
  );
});

Input.displayName = "Input";

export { Input };
