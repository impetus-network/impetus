const footerColumns = [
  { heading: "Network", items: ["Stats", "Validators", "Governance", "Status"] },
  { heading: "Build", items: ["Docs", "RPC endpoints", "SDK", "GitHub"] },
  { heading: "Use", items: ["Bridge", "Wallet", "Explorer", "Faucet"] },
  { heading: "Company", items: ["About", "Blog", "Brand", "Contact"] },
];

export function Footer() {
  return (
    <footer className="bg-[#faf5e8]">
      <div className="mx-auto max-w-7xl px-8 pb-10 pt-16">
        <div className="grid gap-8 lg:grid-cols-[1.5fr_repeat(4,1fr)]">
          <div>
            <div className="flex items-center gap-2.5">
              <span className="flex size-7 items-center justify-center rounded-lg bg-[#0a0a0a] text-xs font-bold text-white">
                A
              </span>
              <span className="text-lg font-semibold tracking-tight text-[#0a0a0a]">
                Artemis
              </span>
            </div>
            <p className="mt-3 max-w-60 text-sm text-[#6a6a6a]">
              The gasless EVM Layer 1.
            </p>
          </div>

          {footerColumns.map((column) => (
            <div key={column.heading}>
              <h2 className="art-caption text-[#6a6a6a]">{column.heading}</h2>
              <ul className="mt-3.5 space-y-2">
                {column.items.map((item) => (
                  <li key={item}>
                    <span className="cursor-pointer text-[13px] text-[#3a3a3a]">
                      {item}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <div className="mt-12 flex flex-col gap-2 border-t border-[#e5e5e5] pt-5 text-xs text-[#6a6a6a] sm:flex-row sm:items-center sm:justify-between">
          <span>© 2026 Artemis Labs · chainId 322</span>
          <span className="font-mono">
            RPC: rpc.artemis.io · WS: wss.artemis.io
          </span>
        </div>
      </div>
    </footer>
  );
}
