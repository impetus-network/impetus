"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { ConnectButtonCustom } from "~/components/scaffold/ConnectButtonCustom";
import { cn } from "~/lib/utils";

const navItems = [
  { href: "/", label: "Home" },
  { href: "/transfer", label: "Transfer" },
  { href: "/blockexplorer", label: "Explorer" },
];

function isActivePath(pathname: string, href: string): boolean {
  return href === "/" ? pathname === "/" : pathname === href || pathname.startsWith(`${href}/`);
}

export function Header() {
  const pathname = usePathname();
  const { address, isConnected } = useAccount();
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  const allNavItems = isAdmin
    ? [...navItems, { href: "/admin/gasless", label: "Admin" }]
    : navItems;

  return (
    <header className="sticky top-0 z-40 border-b border-[#f0f0f0] bg-[#fffaf0]">
      <nav className="flex h-16 items-center gap-8 px-8">
        <Link href="/" className="flex shrink-0 items-center gap-2.5">
          <span className="flex size-7 items-center justify-center rounded-lg bg-[#0a0a0a] text-xs font-bold text-white">
            A
          </span>
          <span className="text-lg font-semibold tracking-tight text-[#0a0a0a]">
            Artemis
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

        <div className="ml-auto flex items-center gap-5">
          <span className="hidden text-sm font-medium text-[#6a6a6a] lg:inline">
            Docs
          </span>
          <span className="hidden text-sm font-medium text-[#6a6a6a] lg:inline">
            Bridge
          </span>
          <ConnectButtonCustom />
        </div>
      </nav>
    </header>
  );
}
