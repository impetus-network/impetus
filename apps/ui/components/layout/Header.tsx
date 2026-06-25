"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import {
  Menu,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from "@artemis/coss-ui/ui/menu";
import { ConnectButtonCustom } from "~/components/scaffold/ConnectButtonCustom";
import { cn } from "~/lib/utils";

interface NavItem {
  href: string;
  label: string;
}

// Items kept inline on desktop. Keep this list short so the bar never overflows.
const primaryNavItems: NavItem[] = [
  { href: "/", label: "Home" },
  { href: "/transfer", label: "Transfer" },
  { href: "/staking", label: "Staking" },
  { href: "/pools", label: "Pools" },
  { href: "/validators", label: "Validators" },
];

// Items collapsed into the "More" dropdown on desktop.
const secondaryNavItems: NavItem[] = [
  { href: "/validator", label: "Run a node" },
  { href: "/blockexplorer", label: "Explorer" },
];

// Placeholder destinations that are not wired up yet.
const placeholderItems: { label: string }[] = [
  { label: "Docs" },
  { label: "Bridge" },
];

function isActivePath(pathname: string, href: string): boolean {
  return href === "/" ? pathname === "/" : pathname === href || pathname.startsWith(`${href}/`);
}

const itemBase = "rounded-lg px-3.5 py-2 text-sm font-medium transition-colors";
const itemInactive = "text-[#6a6a6a] hover:bg-[#f5f0e0] hover:text-[#0a0a0a]";
const itemActive = "bg-[#f5f0e0] text-[#0a0a0a]";

export function Header() {
  const pathname = usePathname();
  const { address, isConnected } = useAccount();
  const [mobileOpen, setMobileOpen] = useState(false);
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  const adminItems: NavItem[] = isAdmin ? [{ href: "/admin/gasless", label: "Admin" }] : [];

  // Everything that lives behind the desktop "More" dropdown.
  const moreItems: NavItem[] = [...secondaryNavItems, ...adminItems];

  // Full list for the mobile drawer (primary + secondary + admin).
  const mobileNavItems: NavItem[] = [...primaryNavItems, ...moreItems];

  const isMoreActive = moreItems.some((item) => isActivePath(pathname, item.href));

  // Close the mobile menu whenever the route changes (link tap, back/forward).
  useEffect(() => {
    setMobileOpen(false);
  }, [pathname]);

  return (
    <header className="sticky top-0 z-40 border-b border-[#f0f0f0] bg-[#fffaf0]">
      <nav className="flex h-16 items-center gap-4 px-4 sm:px-8 xl:gap-8">
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
          {primaryNavItems.map((item) => {
            const isActive = isActivePath(pathname, item.href);

            return (
              <Link
                key={item.href}
                href={item.href}
                aria-current={isActive ? "page" : undefined}
                className={cn(itemBase, isActive ? itemActive : itemInactive)}
              >
                {item.label}
              </Link>
            );
          })}

          <Menu>
            <MenuTrigger
              className={cn(
                itemBase,
                "flex items-center gap-1 outline-none",
                isMoreActive ? itemActive : itemInactive,
              )}
            >
              More
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                aria-hidden="true"
                className="transition-transform"
              >
                <path
                  d="M6 9l6 6 6-6"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </MenuTrigger>
            <MenuPopup align="end" className="min-w-44">
              {moreItems.map((item) => {
                const isActive = isActivePath(pathname, item.href);

                return (
                  <MenuItem
                    key={item.href}
                    render={<Link href={item.href} />}
                    aria-current={isActive ? "page" : undefined}
                    className={cn(isActive && "bg-[#f5f0e0] text-[#0a0a0a]")}
                  >
                    {item.label}
                  </MenuItem>
                );
              })}
              <MenuSeparator />
              {placeholderItems.map((item) => (
                <MenuItem key={item.label} disabled>
                  {item.label}
                  <span className="ms-auto text-[10px] text-[#9a9a9a]">Soon</span>
                </MenuItem>
              ))}
            </MenuPopup>
          </Menu>
        </div>

        <div className="ml-auto flex items-center gap-3 sm:gap-5">
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
            {mobileNavItems.map((item) => {
              const isActive = isActivePath(pathname, item.href);

              return (
                <Link
                  key={item.href}
                  href={item.href}
                  aria-current={isActive ? "page" : undefined}
                  className={cn(
                    "rounded-lg px-4 py-3 text-base font-medium transition-colors",
                    isActive ? itemActive : itemInactive,
                  )}
                >
                  {item.label}
                </Link>
              );
            })}
          </div>
          <div className="mt-2 flex gap-5 border-t border-[#f0f0f0] px-4 pt-3 text-sm font-medium text-[#6a6a6a]">
            {placeholderItems.map((item) => (
              <span key={item.label}>{item.label}</span>
            ))}
          </div>
        </div>
      )}
    </header>
  );
}
