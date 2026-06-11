"use client";

import type { ReactElement } from "react";
import { Badge, type BadgeProps } from "../ui/badge";
import { Button, type ButtonProps } from "../ui/button";
import { cn } from "../utils";

export function ClayButton({
  className,
  ...props
}: ButtonProps): ReactElement {
  return (
    <Button
      className={cn(
        "clay-shadow rounded-full border-[#0a0a0a] font-black",
        className,
      )}
      {...props}
    />
  );
}

export function ClayBadge({
  className,
  ...props
}: BadgeProps): ReactElement {
  return (
    <Badge
      className={cn("rounded-full px-3 py-1 font-black", className)}
      {...props}
    />
  );
}
