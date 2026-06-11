"use client";

import { ExplorerPanels } from "~/components/dapp/ExplorerPanels";
import { useLiveFeed } from "~/components/dapp/LiveFeed";

export default function BlockExplorer() {
  const feed = useLiveFeed();

  return (
    <main className="min-h-screen overflow-x-hidden bg-[#fffaf0] text-[#0a0a0a]">
      <section className="mx-auto max-w-7xl px-4 py-12 sm:px-8 sm:py-16">
        <p className="art-caption text-[#1a3a3a]">Explorer</p>
        <h1 className="art-display mt-3 text-5xl leading-none sm:text-6xl">
          Search the chain.
        </h1>
        <div className="h-4" />
        <ExplorerPanels feed={feed} />
      </section>
    </main>
  );
}
