import type { ComponentPropsWithoutRef, ReactElement, ReactNode } from "react";
import type { DemoToken, TxKind } from "./types";
import { cn } from "~/lib/utils";

const txBadgeClasses: Record<TxKind, string> = {
  transfer: "bg-[#a4d4c5] text-[#0a0a0a]",
  swap: "bg-[#ff4d8b] text-white",
  mint: "bg-[#b8a4ed] text-[#0a0a0a]",
  contract: "bg-[#e8b94a] text-[#0a0a0a]",
};

const cardToneClasses = {
  cream: "bg-[#f5f0e0] text-[#0a0a0a]",
  lavender: "bg-[#b8a4ed] text-[#0a0a0a]",
  ochre: "bg-[#e8b94a] text-[#0a0a0a]",
  peach: "bg-[#ffb084] text-[#0a0a0a]",
  pink: "bg-[#ff4d8b] text-white",
  teal: "bg-[#1a3a3a] text-white",
} as const;

type DappPanelProps = ComponentPropsWithoutRef<"div">;

type MonoProps = {
  children: ReactNode;
  className?: string;
};

type OverviewCardProps = {
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  tone: keyof typeof cardToneClasses;
};

type PulseDotProps = {
  color?: string;
};

type TokenIconProps = {
  token: DemoToken;
  size?: number;
};

type TransactionTypeBadgeProps = {
  type: TxKind;
};

export function PulseDot({ color = "#22c55e" }: PulseDotProps): ReactElement {
  return (
    <span className="relative flex size-3" aria-hidden="true">
      <span
        className="absolute inline-flex h-full w-full rounded-full motion-safe:[animation:artPulse_1.4s_ease-out_infinite]"
        style={{ backgroundColor: color }}
      />
      <span
        className="relative inline-flex size-3 rounded-full"
        style={{ backgroundColor: color }}
      />
    </span>
  );
}

export function Mono({ children, className }: MonoProps): ReactElement {
  return (
    <span className={cn("font-mono tabular-nums tracking-normal", className)}>
      {children}
    </span>
  );
}

export function DappPanel({
  className,
  children,
  ...props
}: DappPanelProps): ReactElement {
  return (
    <div {...props} className={cn("art-panel", className)}>
      {children}
    </div>
  );
}

export function OverviewCard({
  label,
  value,
  sub,
  tone,
}: OverviewCardProps): ReactElement {
  return (
    <article
      className={cn(
        "min-h-36 rounded-3xl p-5 shadow-[inset_0_0_0_1px_rgba(10,10,10,0.08)]",
        cardToneClasses[tone],
      )}
    >
      <p className="art-caption opacity-70">{label}</p>
      <div className="art-display mt-5 text-4xl leading-none">{value}</div>
      {sub && <p className="mt-3 text-sm font-medium opacity-70">{sub}</p>}
    </article>
  );
}

export function TransactionTypeBadge({
  type,
}: TransactionTypeBadgeProps): ReactElement {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-sm px-2 py-0.5 text-[9px] font-semibold uppercase leading-none tracking-wider",
        txBadgeClasses[type],
      )}
    >
      {type}
    </span>
  );
}

export function TokenIcon({ token, size = 32 }: TokenIconProps): ReactElement {
  const letters = token.sym.slice(0, 2).toUpperCase();

  return (
    <span
      className="inline-flex shrink-0 items-center justify-center rounded-full border border-[#0a0a0a]/10 font-black text-[#0a0a0a] shadow-[inset_0_1px_0_rgba(255,255,255,0.35)]"
      style={{
        backgroundColor: token.color,
        fontSize: Math.max(10, Math.round(size * 0.32)),
        height: size,
        width: size,
      }}
      title={token.name}
    >
      {letters}
    </span>
  );
}
