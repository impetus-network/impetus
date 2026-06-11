"use client";

import { createContext, useCallback, useContext, useRef, useState } from "react";
import { formatEther, formatGwei } from "viem";
import {
  Dialog,
  DialogPopup,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogPanel,
  DialogFooter,
  DialogClose,
} from "@artemis/coss-ui/ui/dialog";
import { Button } from "@artemis/coss-ui/ui/button";
import type { GasEstimate, SimpleTxParams } from "~/hooks/useTransactor";

export interface TxConfirmDetails {
  type: "transfer" | "contract";
  to?: string;
  value?: bigint;
  functionName?: string;
  gasEstimate?: GasEstimate;
}

type ConfirmFn = (details: TxConfirmDetails) => Promise<boolean>;

const TxConfirmContext = createContext<ConfirmFn | null>(null);

export function useTxConfirm(): ConfirmFn {
  const confirm = useContext(TxConfirmContext);
  if (!confirm) throw new Error("useTxConfirm must be used within TxConfirmProvider");
  return confirm;
}

function truncateAddr(addr: string): string {
  return `${addr.slice(0, 10)}...${addr.slice(-8)}`;
}

export function TxConfirmProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  const [details, setDetails] = useState<TxConfirmDetails | null>(null);
  const resolveRef = useRef<((value: boolean) => void) | null>(null);

  const confirm: ConfirmFn = useCallback((txDetails) => {
    setDetails(txDetails);
    setOpen(true);
    return new Promise<boolean>((resolve) => {
      resolveRef.current = resolve;
    });
  }, []);

  function handleConfirm() {
    setOpen(false);
    resolveRef.current?.(true);
    resolveRef.current = null;
  }

  function handleCancel() {
    setOpen(false);
    resolveRef.current?.(false);
    resolveRef.current = null;
  }

  return (
    <TxConfirmContext.Provider value={confirm}>
      {children}
      <Dialog open={open} onOpenChange={(nextOpen) => { if (!nextOpen) handleCancel(); }}>
        <DialogPopup showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Confirm Transaction</DialogTitle>
            <DialogDescription>
              Review the details before submitting to your wallet.
            </DialogDescription>
          </DialogHeader>
          <DialogPanel className="p-6 pt-0">
            {details && (
              <div className="flex flex-col gap-3">
                {details.type === "transfer" && details.to && (
                  <Row label="To" value={truncateAddr(details.to)} mono />
                )}
                {details.type === "contract" && details.functionName && (
                  <Row label="Function" value={details.functionName} mono />
                )}
                {details.type === "contract" && details.to && (
                  <Row label="Contract" value={truncateAddr(details.to)} mono />
                )}
                {details.value !== undefined && details.value > 0n && (
                  <Row label="Value" value={`${formatEther(details.value)} ART`} />
                )}
                {details.gasEstimate && (
                  <>
                    <div className="border-t border-border" />
                    <Row label="Estimated Gas" value={details.gasEstimate.formatted.gas} />
                    <Row label="Gas Price" value={details.gasEstimate.formatted.gasPrice} />
                    <Row label="Estimated Fee" value={details.gasEstimate.formatted.totalCost} highlight />
                  </>
                )}
              </div>
            )}
          </DialogPanel>
          <DialogFooter className="flex gap-2 p-6 pt-0">
            <DialogClose render={<Button variant="outline" className="flex-1" onClick={handleCancel} />}>
              Cancel
            </DialogClose>
            <Button className="flex-1" onClick={handleConfirm}>
              Confirm
            </Button>
          </DialogFooter>
        </DialogPopup>
      </Dialog>
    </TxConfirmContext.Provider>
  );
}

function Row({ label, value, mono, highlight }: { label: string; value: string; mono?: boolean; highlight?: boolean }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-muted-foreground">{label}</span>
      <span className={`text-sm ${mono ? "font-mono" : ""} ${highlight ? "font-medium" : ""}`}>
        {value}
      </span>
    </div>
  );
}
