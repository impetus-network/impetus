"use client";

import Link from "next/link";
import { useConnectModal } from "@rainbow-me/rainbowkit";
import { useAccount } from "wagmi";
import { NetworkFeedCard, useLiveFeed } from "~/components/dapp/LiveFeed";
import { Mono, PulseDot } from "~/components/dapp/DappPrimitives";
import { cn } from "~/lib/utils";

const compatibilityItems = ["MetaMask", "Hardhat", "Foundry", "Viem"];

const stats = [
  { label: "Block time", value: "6s", sub: "BABE slot cadence", accent: "text-[#6a6a6a]" },
  { label: "Finality", value: "~12s", sub: "GRANDPA, deterministic", accent: "text-[#6a6a6a]" },
  { label: "Validators", value: "5", sub: "+ archive node", accent: "text-[#6a6a6a]" },
  { label: "Gas paid by users", value: "$0.00", sub: "Always.", accent: "text-[#ff4d8b] font-semibold" },
];


function StatStrip() {
  return (
    <section className="px-4 pb-24 sm:px-8">
      <div className="mx-auto max-w-7xl">
        <div className="grid grid-cols-2 overflow-hidden rounded-3xl bg-[#faf5e8] lg:grid-cols-4">
          {stats.map((stat, i) => (
            <article
              className={cn(
                "min-w-0 p-5 sm:p-6",
                i < stats.length - 1 && "border-r border-[#e5e5e5]",
              )}
              key={stat.label}
            >
              <p className="art-caption break-words text-[#6a6a6a]">
                {stat.label}
              </p>
              <p className="art-display mt-4 text-3xl leading-none text-[#0a0a0a] sm:text-4xl">
                <Mono>{stat.value}</Mono>
              </p>
              <p className={cn("mt-2 font-mono text-xs", stat.accent)}>
                {stat.sub}
              </p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function ReceiptPreview() {
  const rows = [
    { label: "Send 50 USDC", value: "✓", highlight: false },
    { label: "Network fee", value: "0.00 IPT", highlight: true },
    { label: "Wallet balance Δ", value: "−50.00 USDC", highlight: false },
  ];
  return (
    <div className="rounded-md bg-white/95 p-3.5 text-[#0a0a0a]">
      {rows.map((row, i) => (
        <div
          className={cn(
            "flex items-center justify-between py-2",
            i > 0 && "border-t border-[#f0f0f0]",
          )}
          key={row.label}
        >
          <span className="text-xs font-medium">{row.label}</span>
          <Mono
            className={cn(
              "text-xs",
              row.highlight
                ? "font-semibold text-[#ff4d8b]"
                : "text-[#6a6a6a]",
            )}
          >
            {row.value}
          </Mono>
        </div>
      ))}
    </div>
  );
}

function TerminalPreview() {
  return (
    <div className="rounded-md bg-[#0a1a1a] p-4 font-mono text-xs leading-relaxed text-white">
      <div>
        <span className="text-[#9a9a9a]">$ </span>
        <span>forge create Token --rpc-url impetus</span>
      </div>
      <div className="opacity-70">
        <span className="text-[#9a9a9a]">&gt; </span>
        <span>Deployed to 0x9c2…1f4</span>
      </div>
      <div className="text-[#a4d4c5] opacity-70">
        <span className="text-[#a4d4c5]">&gt; </span>
        <span>Gas used: 0</span>
      </div>
    </div>
  );
}

function FinalityChart() {
  const times = [0.42, 0.39, 0.41, 0.38, 0.40, 0.43, 0.39, 0.41, 0.40, 0.42, 0.38, 0.41];
  return (
    <div aria-hidden="true" className="rounded-md bg-white/50 p-3.5">
      <div className="mb-2 flex h-16 items-end gap-1">
        {times.map((t, i) => (
          <div
            className="min-w-0 flex-1 rounded-t bg-[#0a0a0a]"
            key={`${t}-${i}`}
            style={{
              height: `${(t / 0.5) * 100}%`,
              opacity: 0.4 + (i / times.length) * 0.6,
            }}
          />
        ))}
      </div>
      <div className="flex justify-between font-mono text-[11px] text-[#0a0a0a]/70">
        <span>block: ~6s</span>
        <span>finality: ~12s</span>
      </div>
    </div>
  );
}

function FeatureGrid() {
  return (
    <section className="px-4 pb-24 sm:px-8">
      <div className="mx-auto max-w-7xl">
        <div className="mb-8 flex items-baseline justify-between">
          <h2 className="art-display max-w-2xl text-3xl leading-[1.1] text-[#0a0a0a] sm:text-4xl">
            Built for the chains the rest of crypto promised.
          </h2>
          <span className="hidden text-sm text-[#6a6a6a] underline underline-offset-2 lg:inline">
            See architecture →
          </span>
        </div>

        <div className="grid gap-4 lg:grid-cols-3">
          <article className="flex min-h-96 flex-col gap-3.5 rounded-3xl bg-[#ff4d8b] p-7 text-white">
            <p className="art-caption text-white/70">Gasless</p>
            <h3 className="art-display text-[1.75rem] leading-[1.15]">
              Users never pay for gas. Ever.
            </h3>
            <p className="text-sm leading-relaxed text-white/85">
              Apps sponsor execution by default. Onboard a Web2 user without
              explaining wei.
            </p>
            <div className="mt-auto">
              <ReceiptPreview />
            </div>
          </article>

          <article className="flex min-h-96 flex-col gap-3.5 rounded-3xl bg-[#b8a4ed] p-7 text-[#0a0a0a]">
            <p className="art-caption text-[#0a0a0a]/70">EVM compatible</p>
            <h3 className="art-display text-[1.75rem] leading-[1.15]">
              Deploy your Solidity, untouched.
            </h3>
            <p className="text-sm leading-relaxed text-[#0a0a0a]/85">
              Bytecode-level compatibility with Ethereum. Your existing tooling
              — Hardhat, Foundry, Viem — works without changes.
            </p>
            <div className="mt-auto">
              <TerminalPreview />
            </div>
          </article>

          <article className="flex min-h-96 flex-col gap-3.5 rounded-3xl bg-[#e8b94a] p-7 text-[#0a0a0a]">
            <p className="art-caption text-[#0a0a0a]/60">Deterministic finality</p>
            <h3 className="art-display text-[1.75rem] leading-[1.15]">
              6s blocks. GRANDPA finality.
            </h3>
            <p className="text-sm leading-relaxed text-[#0a0a0a]/85">
              BABE block production with GRANDPA deterministic finality. Your
              transaction settles in seconds, then never reverts.
            </p>
            <div className="mt-auto">
              <FinalityChart />
            </div>
          </article>
        </div>
      </div>
    </section>
  );
}

function DeveloperBand() {
  return (
    <section className="px-4 pb-24 sm:px-8">
      <div className="mx-auto grid max-w-7xl overflow-hidden rounded-3xl bg-[#f5f0e0] lg:grid-cols-2">
        <div className="flex flex-col justify-between gap-10 p-10 lg:p-14">
          <div>
            <p className="art-caption text-[#6a6a6a]">For developers</p>
            <h2 className="art-display mt-4 text-3xl leading-none sm:text-4xl">
              Point your RPC at us. Done.
            </h2>
            <p className="mt-5 max-w-md text-base leading-7 text-[#3a3a3a]">
              No new tooling, no new language, no porting headache. If it runs
              on Ethereum, it runs on Impetus — bytecode-compatible, with no
              fee surface to manage.
            </p>
          </div>

          <div className="flex flex-col gap-3 sm:flex-row">
            <span className="inline-flex min-h-12 items-center justify-center rounded-md bg-[#0a0a0a] px-6 text-sm font-semibold text-white transition hover:bg-[#1f1f1f]">
              Read the docs
            </span>
            <span className="inline-flex min-h-12 items-center justify-center rounded-md border border-[#e5e5e5] px-6 text-sm font-semibold text-[#0a0a0a]">
              View on GitHub
            </span>
          </div>
        </div>

        <div className="flex flex-col gap-4 bg-[#0a1a1a] p-8 text-white">
          <div className="flex items-center gap-1.5">
            <span className="size-2.5 rounded-full bg-[#ff6b5a]" />
            <span className="size-2.5 rounded-full bg-[#e8b94a]" />
            <span className="size-2.5 rounded-full bg-[#a4d4c5]" />
            <span className="ml-auto font-mono text-[11px] text-[#9a9a9a]">
              impetus.config.ts
            </span>
          </div>
          <pre className="overflow-x-auto whitespace-pre font-mono text-sm leading-7 text-white">
            <code>
              <span className="text-[#a4d4c5]">import</span>
              {" { defineConfig } "}
              <span className="text-[#a4d4c5]">from</span>
              {" "}
              <span className="text-[#ffb084]">&quot;viem&quot;</span>
              {";\n\n"}
              <span className="text-[#a4d4c5]">export default</span>
              {" defineConfig({\n"}
              {"  chain: "}
              <span className="text-[#ffb084]">&quot;impetus&quot;</span>
              {",\n"}
              {"  rpcUrl: "}
              <span className="text-[#ffb084]">
                &quot;https://rpc-proxy-production-a44c.up.railway.app&quot;
              </span>
              {",\n"}
              {"  chainId: "}
              <span className="text-[#b8a4ed]">388266</span>
              {",\n"}
              {"  gasless: "}
              <span className="text-[#b8a4ed]">true</span>
              {",\n});"}</code>
          </pre>
          <div className="mt-auto flex gap-3.5 border-t border-white/10 pt-4 font-mono text-[11px] text-[#9a9a9a]">
            <span>chainId: 388266</span>
            <span>·</span>
            <span>finality: ~12s</span>
            <span>·</span>
            <span>fee: 0</span>
          </div>
        </div>
      </div>
    </section>
  );
}

export default function Home() {
  const { isConnected } = useAccount();
  const { openConnectModal } = useConnectModal();
  const feed = useLiveFeed();

  return (
    <div className="bg-[#fffaf0] text-[#0a0a0a]">
      <section className="px-4 py-14 sm:px-8 sm:py-20 lg:py-24">
        <div className="mx-auto grid max-w-7xl items-center gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,0.82fr)]">
          <div className="min-w-0">
            <div className="art-caption flex items-center gap-2.5 text-[#6a6a6a]">
              <PulseDot color="#22c55e" />
              Mainnet · Live
            </div>

            <h1 className="art-display mt-7 max-w-4xl text-6xl leading-[0.88] tracking-normal text-[#0a0a0a] sm:text-7xl lg:text-8xl">
              The gasless EVM chain.
            </h1>

            <p className="mt-7 max-w-xl text-lg font-normal leading-7 text-[#3a3a3a] sm:text-xl sm:leading-8">
              Impetus is an EVM-compatible Layer 1 with zero gas fees. Deploy
              your existing contracts, transfer instantly, settle in seconds
              — without paying for every byte.
            </p>

            <div className="mt-9 flex flex-col gap-3 sm:flex-row">
              {isConnected ? (
                <Link
                  className="inline-flex min-h-12 items-center justify-center rounded-md bg-[#0a0a0a] px-6 text-sm font-semibold text-white transition hover:bg-[#1f1f1f]"
                  href="/transfer"
                >
                  Open transfer
                </Link>
              ) : (
                <button
                  className="inline-flex min-h-12 items-center justify-center rounded-md bg-[#0a0a0a] px-6 text-sm font-semibold text-white transition hover:bg-[#1f1f1f]"
                  onClick={() => openConnectModal?.()}
                  type="button"
                >
                  Connect wallet
                </button>
              )}
              <span className="inline-flex min-h-12 items-center justify-center rounded-md border border-[#e5e5e5] bg-[#fffaf0] px-6 text-sm font-semibold text-[#0a0a0a] transition hover:border-[#0a0a0a]/25">
                Read the whitepaper →
              </span>
            </div>

            <div className="mt-10 flex flex-wrap items-center gap-4 text-sm text-[#6a6a6a]">
              <span>Compatible with</span>
              {compatibilityItems.map((item) => (
                <Mono className="text-[#0a0a0a]" key={item}>
                  {item}
                </Mono>
              ))}
            </div>
          </div>

          <div className="min-w-0">
            <NetworkFeedCard feed={feed} />
          </div>
        </div>
      </section>

      <StatStrip />
      <FeatureGrid />
      <DeveloperBand />
    </div>
  );
}
