import Link from "next/link";

const NAV_ITEMS = [
  { href: "/block", label: "Blocks" },
  { href: "/tx", label: "Transactions" },
  { href: "/address", label: "Addresses" },
  { href: "/contract", label: "Contracts" },
] as const;

export function Nav() {
  return (
    <nav className="flex items-center gap-6" aria-label="Main navigation">
      {NAV_ITEMS.map((item) => (
        <Link
          key={item.href}
          href={item.href}
          className="text-sm font-medium text-gray-600 hover:text-gray-900 transition-colors"
        >
          {item.label}
        </Link>
      ))}
    </nav>
  );
}
