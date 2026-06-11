"use client";

import type {
  ComponentPropsWithoutRef,
  ReactElement,
  ReactNode,
} from "react";
import { cn } from "../utils";

type DivRootProps = ComponentPropsWithoutRef<"div">;

type ClayTableFrameProps = Omit<DivRootProps, "children"> & {
  children: ReactNode;
};

type ClayEmptyStateProps = Omit<DivRootProps, "children" | "title"> & {
  title: string;
  description?: string;
};

export function ClayTableFrame({
  className,
  children,
  ...props
}: ClayTableFrameProps): ReactElement {
  return (
    <div
      {...props}
      className={cn(
        "clay-border overflow-x-auto rounded-[1.25rem] border bg-white shadow",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function ClayEmptyState({
  title,
  description,
  className,
  ...props
}: ClayEmptyStateProps): ReactElement {
  return (
    <div
      {...props}
      className={cn(
        "clay-border clay-shadow rounded-[1.5rem] border bg-[#fffaf0] p-8 text-center",
        className,
      )}
    >
      <h3 className="clay-ink text-xl font-black leading-tight">{title}</h3>
      {description && (
        <p className="mx-auto mt-2 max-w-md text-sm font-medium text-[#6a6a6a]">
          {description}
        </p>
      )}
    </div>
  );
}
