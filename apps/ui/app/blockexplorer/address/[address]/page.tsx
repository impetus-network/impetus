"use client";

import { useParams } from "next/navigation";
import { type Address as AddressType } from "viem";
import { Address } from "~/components/scaffold/Address";
import { Balance } from "~/components/scaffold/Balance";

export default function AddressPage() {
  const { address } = useParams<{ address: string }>();
  const addr = address as AddressType;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Address</h1>
      <div className="rounded-lg border border-border p-6">
        <dl className="grid gap-4">
          <div>
            <dt className="text-sm text-muted-foreground">Address</dt>
            <dd><Address address={addr} format="full" /></dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Balance</dt>
            <dd className="text-xl font-bold"><Balance address={addr} /></dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
