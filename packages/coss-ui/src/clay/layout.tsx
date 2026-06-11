"use client";

import type {
  ComponentProps,
  ComponentPropsWithoutRef,
  ReactElement,
  ReactNode,
} from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { cn } from "../utils";

type DivRootProps = ComponentPropsWithoutRef<"div">;
type SectionRootProps = ComponentPropsWithoutRef<"section">;

type ClayBaseProps = Omit<DivRootProps, "children"> & {
  children: ReactNode;
};

type ClayHeroProps = Omit<SectionRootProps, "children" | "title"> & {
  eyebrow?: string;
  title: string;
  description?: string;
  children?: ReactNode;
};

type ClaySectionProps = Omit<SectionRootProps, "children" | "title"> & {
  title: string;
  description?: string;
  children: ReactNode;
  action?: ReactNode;
};

type ClayCardProps = Omit<
  ComponentProps<typeof Card>,
  "children" | "title"
> & {
  title?: string;
  description?: string;
  children: ReactNode;
};

type ClayFeatureTone =
  | "pink"
  | "teal"
  | "lavender"
  | "peach"
  | "ochre"
  | "cream";

type ClayFeatureCardProps = Omit<DivRootProps, "children"> & {
  label: string;
  value: ReactNode;
  tone?: ClayFeatureTone;
  children?: ReactNode;
};

const featureToneClasses: Record<ClayFeatureTone, string> = {
  pink: "bg-[#ff4d8b] text-white",
  teal: "bg-[#1a3a3a] text-white",
  lavender: "bg-[#b8a4ed] text-[#0a0a0a]",
  peach: "bg-[#ffb084] text-[#0a0a0a]",
  ochre: "bg-[#e8b94a] text-[#0a0a0a]",
  cream: "bg-[#fffaf0] text-[#0a0a0a]",
};

export function ClayPage({
  className,
  children,
  ...props
}: ClayBaseProps): ReactElement {
  return (
    <div {...props} className={cn("flex flex-col gap-8", className)}>
      {children}
    </div>
  );
}

export function ClayHero({
  eyebrow,
  title,
  description,
  children,
  className,
  ...props
}: ClayHeroProps): ReactElement {
  return (
    <section
      {...props}
      className={cn(
        "clay-border clay-shadow relative overflow-hidden rounded-[2rem] border bg-[#fffaf0] p-6 sm:p-8",
        className,
      )}
    >
      <div
        aria-hidden="true"
        className="absolute -right-8 -top-8 hidden size-32 rounded-full bg-[#ff4d8b]/25 sm:block"
      />
      <div
        aria-hidden="true"
        className="absolute -bottom-10 right-20 hidden size-28 rotate-6 rounded-[1.5rem] bg-[#b8a4ed]/35 sm:block"
      />
      <div className="relative z-10 flex max-w-4xl flex-col gap-5">
        {eyebrow && (
          <p className="text-sm font-black uppercase tracking-[0.16em] text-[#1a3a3a]">
            {eyebrow}
          </p>
        )}
        <h1 className="clay-ink text-4xl font-black leading-none sm:text-6xl">
          {title}
        </h1>
        {description && (
          <p className="max-w-2xl text-lg font-medium leading-7 text-[#3a3a3a]">
            {description}
          </p>
        )}
        {children}
      </div>
    </section>
  );
}

export function ClaySection({
  title,
  description,
  children,
  action,
  className,
  ...props
}: ClaySectionProps): ReactElement {
  return (
    <section {...props} className={cn("flex flex-col gap-4", className)}>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h2 className="clay-ink text-2xl font-black leading-tight">
            {title}
          </h2>
          {description && (
            <p className="max-w-2xl text-sm font-medium text-[#6a6a6a]">
              {description}
            </p>
          )}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </div>
      {children}
    </section>
  );
}

export function ClayPanel({
  className,
  children,
  ...props
}: ClayBaseProps): ReactElement {
  return (
    <div
      {...props}
      className={cn(
        "clay-border clay-shadow rounded-[1.5rem] border bg-[#fffaf0] p-5",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function ClayCard({
  title,
  description,
  children,
  className,
  ...props
}: ClayCardProps): ReactElement {
  const hasHeader = Boolean(title || description);

  return (
    <Card
      {...props}
      className={cn(
        "clay-border clay-shadow overflow-hidden rounded-[1.5rem] bg-[#fffaf0]",
        className,
      )}
    >
      {hasHeader && (
        <CardHeader>
          {title && <CardTitle>{title}</CardTitle>}
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
      )}
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export function ClayFeatureCard({
  label,
  value,
  tone = "cream",
  children,
  className,
  ...props
}: ClayFeatureCardProps): ReactElement {
  return (
    <div
      {...props}
      className={cn(
        "clay-border clay-shadow rounded-[1.5rem] border p-5",
        featureToneClasses[tone],
        className,
      )}
    >
      <p className="text-sm font-black uppercase tracking-[0.14em] opacity-75">
        {label}
      </p>
      <p className="mt-3 text-3xl font-black leading-none">{value}</p>
      {children && <div className="mt-4 text-sm font-medium">{children}</div>}
    </div>
  );
}
