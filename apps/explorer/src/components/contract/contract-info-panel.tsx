import Link from "next/link";
import { formatNumber } from "@artemis/shared";
import { Badge } from "@artemis/coss-ui/ui/badge";
import { Separator } from "@artemis/coss-ui/ui/separator";
import { AddressLink } from "@/components/shared/address-link";
import { HashLink } from "@/components/shared/hash-link";
import { CopyButton } from "@/components/shared/copy-button";
import { JsonViewer } from "@/components/shared/json-viewer";

interface ContractDetail {
  address: string;
  name: string | null;
  txCount: number;
  verified: boolean;
  deployer: string | null;
  txHash: string | null;
  abi: string | null;
  sourceCode: string | null;
  bytecode: string | null;
  compilerVersion: string | null;
  evmVersion: string | null;
}

interface ContractInfoPanelProps {
  contract: ContractDetail;
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

export function ContractInfoPanel({ contract }: ContractInfoPanelProps) {
  return (
    <dl className="divide-y divide-gray-100">
      <Row label="Address">
        <span className="font-mono text-xs">{contract.address}</span>
        <CopyButton text={contract.address} />
      </Row>
      <Row label="Name">
        {contract.name ?? <span className="text-gray-400">--</span>}
      </Row>
      <Row label="Verified">
        {contract.verified ? (
          <Badge variant="success" size="sm">
            Verified
          </Badge>
        ) : (
          <Badge variant="outline" size="sm">
            Unverified
          </Badge>
        )}
      </Row>
      <Row label="Transactions">{formatNumber(contract.txCount)}</Row>
      <Separator />
      <Row label="Deployer">
        {contract.deployer ? (
          <AddressLink
            address={contract.deployer}
            head={10}
            tail={8}
          />
        ) : (
          <span className="text-gray-400">--</span>
        )}
      </Row>
      <Row label="Creation Tx">
        {contract.txHash ? (
          <HashLink
            hash={contract.txHash}
            href={`/tx/${contract.txHash}`}
            head={10}
            tail={8}
          />
        ) : (
          <span className="text-gray-400">--</span>
        )}
      </Row>
      {contract.compilerVersion && (
        <Row label="Compiler">{contract.compilerVersion}</Row>
      )}
      {contract.evmVersion && (
        <Row label="EVM Version">{contract.evmVersion}</Row>
      )}
      {contract.abi && (
        <>
          <Separator />
          <Row label="ABI">
            <JsonViewer data={contract.abi} label="ABI" />
          </Row>
        </>
      )}
    </dl>
  );
}
