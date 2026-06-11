import { formatBalance, formatNumber } from "@artemis/shared";
import { Separator } from "@artemis/coss-ui/ui/separator";
import { CopyButton } from "@/components/shared/copy-button";

interface AccountDetail {
  address: string;
  balance: string;
  nonce: number;
  locked: string;
  reserved: string;
}

interface AddressInfoPanelProps {
  account: AccountDetail;
}

function Row({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-4 py-3">
      <dt className="w-40 shrink-0 text-sm font-medium text-gray-500">
        {label}
      </dt>
      <dd className="text-sm text-gray-900 break-all">{children}</dd>
    </div>
  );
}

export function AddressInfoPanel({ account }: AddressInfoPanelProps) {
  return (
    <dl className="divide-y divide-gray-100">
      <Row label="Address">
        <span className="font-mono text-xs">{account.address}</span>
        <CopyButton text={account.address} />
      </Row>
      <Separator />
      <Row label="Balance">{formatBalance(account.balance, 18)} ART</Row>
      <Row label="Nonce">{formatNumber(account.nonce)}</Row>
      <Row label="Locked">{formatBalance(account.locked, 18)} ART</Row>
      <Row label="Reserved">
        {formatBalance(account.reserved, 18)} ART
      </Row>
    </dl>
  );
}
