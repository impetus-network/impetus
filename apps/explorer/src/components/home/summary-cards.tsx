import { formatNumber } from "@artemis/shared";

interface ChainMetadata {
  blockHeight: number;
  txCount: number;
  accountCount: number;
  contractCount: number;
}

interface SummaryCardsProps {
  metadata: ChainMetadata;
}

interface CardProps {
  label: string;
  value: string;
}

function StatCard({ label, value }: CardProps) {
  return (
    <div className="rounded-lg border border-gray-200 bg-white p-5">
      <p className="text-sm font-medium text-gray-500">{label}</p>
      <p className="mt-1 text-2xl font-semibold text-gray-900 tabular-nums">
        {value}
      </p>
    </div>
  );
}

export function SummaryCards({ metadata }: SummaryCardsProps) {
  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      <StatCard
        label="Block Height"
        value={formatNumber(metadata.blockHeight)}
      />
      <StatCard
        label="Transactions"
        value={formatNumber(metadata.txCount)}
      />
      <StatCard
        label="Addresses"
        value={formatNumber(metadata.accountCount)}
      />
      <StatCard
        label="Contracts"
        value={formatNumber(metadata.contractCount)}
      />
    </div>
  );
}
