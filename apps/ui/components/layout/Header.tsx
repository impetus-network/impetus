"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { ConnectButtonCustom } from "~/components/scaffold/ConnectButtonCustom";
import { cn } from "~/lib/utils";

const navItems = [
  { href: "/", label: "Home" },
  { href: "/transfer", label: "Transfer" },
  { href: "/staking", label: "Staking" },
  { href: "/pools", label: "Pools" },
  { href: "/validators", label: "Validators" },
  { href: "/validator", label: "Run a node" },
  { href: "/blockexplorer", label: "Explorer" },
];

function isActivePath(pathname: string, href: string): boolean {
  return href === "/" ? pathname === "/" : pathname === href || pathname.startsWith(`${href}/`);
}

export function Header() {
  const pathname = usePathname();
  const { address, isConnected } = useAccount();
  const [mobileOpen, setMobileOpen] = useState(false);
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  const allNavItems = isAdmin
    ? [...navItems, { href: "/admin/gasless", label: "Admin" }]
    : navItems;

  // Close the mobile menu whenever the route changes (link tap, back/forward).
  useEffect(() => {
    setMobileOpen(false);
  }, [pathname]);

  return (
    <header className="sticky top-0 z-40 border-b border-[#f0f0f0] bg-[#fffaf0]">
      <nav className="flex h-16 items-center gap-8 px-4 sm:px-8">
        <Link href="/" className="flex shrink-0 items-center gap-2.5">
          <span className="flex size-7 items-center justify-center rounded-lg bg-[#0a0a0a] text-xs font-bold text-white">
            I
          </span>
          <span className="text-lg font-semibold tracking-tight text-[#0a0a0a]">
            Impetus
          </span>
          <span className="art-caption hidden rounded-sm bg-[#f5f0e0] px-2 py-0.5 text-[10px] text-[#6a6a6a] sm:inline-flex">
            Mainnet
          </span>
        </Link>

        <div className="hidden items-center gap-1 lg:flex">
          {allNavItems.map((item) => {
            const isActive = isActivePath(pathname, item.href);

            return (
              <Link
                key={item.href}
                href={item.href}
                aria-current={isActive ? "page" : undefined}
                className={cn(
                  "rounded-lg px-3.5 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-[#f5f0e0] text-[#0a0a0a]"
                    : "text-[#6a6a6a] hover:bg-[#f5f0e0] hover:text-[#0a0a0a]",
                )}
              >
                {item.label}
              </Link>
            );
          })}
        </div>

        <div className="ml-auto flex items-center gap-3 sm:gap-5">
          <span className="hidden text-sm font-medium text-[#6a6a6a] lg:inline">
            Docs
          </span>
          <span className="hidden text-sm font-medium text-[#6a6a6a] lg:inline">
            Bridge
          </span>
          <ConnectButtonCustom />

          <button
            type="button"
            aria-label={mobileOpen ? "Close menu" : "Open menu"}
            aria-expanded={mobileOpen}
            aria-controls="mobile-nav"
            onClick={() => setMobileOpen((v) => !v)}
            className="flex size-9 items-center justify-center rounded-lg text-[#0a0a0a] transition-colors hover:bg-[#f5f0e0] lg:hidden"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              {mobileOpen ? (
                <path
                  d="M6 6l12 12M18 6L6 18"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              ) : (
                <path
                  d="M4 7h16M4 12h16M4 17h16"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              )}
            </svg>
          </button>
        </div>
      </nav>

      {mobileOpen && (
        <div
          id="mobile-nav"
          className="border-t border-[#f0f0f0] bg-[#fffaf0] px-4 pb-4 pt-2 lg:hidden"
        >
          <div className="flex flex-col gap-1">
            {allNavItems.map((item) => {
              const isActive = isActivePath(pathname, item.href);

              return (
                <Link
                  key={item.href}
                  href={item.href}
                  aria-current={isActive ? "page" : undefined}
                  className={cn(
                    "rounded-lg px-4 py-3 text-base font-medium transition-colors",
                    isActive
                      ? "bg-[#f5f0e0] text-[#0a0a0a]"
                      : "text-[#6a6a6a] hover:bg-[#f5f0e0] hover:text-[#0a0a0a]",
                  )}
                >
                  {item.label}
                </Link>
              );
            })}
          </div>
          <div className="mt-2 flex gap-5 border-t border-[#f0f0f0] px-4 pt-3 text-sm font-medium text-[#6a6a6a]">
            <span>Docs</span>
            <span>Bridge</span>
          </div>
        </div>
      )}
    </header>
  );
}
