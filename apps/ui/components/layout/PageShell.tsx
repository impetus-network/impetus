import type { ReactElement, ReactNode } from "react";

interface PageShellProps {
  eyebrow: string;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
}

// Editorial page shell shared by the dApp pages, modelled on the block
// explorer: a centred max-w-7xl column with an art-caption eyebrow over a large
// art-display heading. AppLayout already provides the <main> landmark, cream
// background and min-height, so this stays a plain <section>.
export function PageShell({
  eyebrow,
  title,
  description,
  actions,
  children,
}: PageShellProps): ReactElement {
  return (
    <section className="mx-auto w-full max-w-7xl px-4 py-12 sm:px-8 sm:py-16">
      <header>
        <p className="art-caption text-[#1a3a3a]">{eyebrow}</p>
        <h1 className="art-display mt-3 text-5xl leading-none sm:text-6xl">
          {title}
        </h1>
        {description && (
          <p className="mt-5 max-w-2xl text-lg font-normal leading-7 text-[#3a3a3a]">
            {description}
          </p>
        )}
        {actions && <div className="mt-5">{actions}</div>}
      </header>
      <div className="mt-10 flex flex-col gap-8">{children}</div>
    </section>
  );
}
