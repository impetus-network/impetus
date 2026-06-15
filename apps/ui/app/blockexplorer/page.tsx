"use client";

import { ExplorerPanels } from "~/components/dapp/ExplorerPanels";
import { useLiveFeed } from "~/components/dapp/LiveFeed";
import { PageShell } from "~/components/layout/PageShell";

export default function BlockExplorer() {
  const feed = useLiveFeed();

  return (
    <PageShell eyebrow="Explorer" title="Search the chain.">
      <ExplorerPanels feed={feed} />
    </PageShell>
  );
}
