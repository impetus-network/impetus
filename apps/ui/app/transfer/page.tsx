"use client";

import { useConnectModal } from "@rainbow-me/rainbowkit";
import { type ReactElement, useEffect, useRef, useState } from "react";
import { formatEther, isAddress, parseEther } from "viem";
import { useAccount, useBalance, usePublicClient } from "wagmi";
import { DappPanel, Mono } from "~/components/dapp/DappPrimitives";
import { PageShell } from "~/components/layout/PageShell";
import { useTransactor } from "~/hooks/useTransactor";
import { useEnsResolve } from "~/hooks/useEnsResolve";

type SuccessState = {
  amount: number;
  token: string;
  hash: `0x${string}`;
};

const amountInputId = "transfer-amount";
const amountErrorId = "transfer-amount-error";
const recipientInputId = "transfer-recipient";
const recipientErrorId = "transfer-recipient-error";

function isValidDecimalAmount(value: string): boolean {
  return /^\d+(?:\.\d+)?$/.test(value);
}

function parseDecimalAmount(value: string): number {
  if (!isValidDecimalAmount(value)) return 0;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function shortHash(value: string): string {
  return `${value.slice(0, 6)}...${value.slice(-4)}`;
}

function formatTokenAmount(value: number): string {
  return value.toLocaleString("en-US", {
    maximumFractionDigits: value >= 100 ? 2 : 4,
  });
}

function getButtonLabel({
  amount,
  balance,
  connected,
  sending,
  to,
  isEnsInput,
  isResolving,
  ensError,
}: {
  amount: number;
  balance: number;
  connected: boolean;
  sending: boolean;
  to: string;
  isEnsInput: boolean;
  isResolving: boolean;
  ensError: string | null;
}): string {
  if (!connected) return "Connect wallet to send";
  if (sending) return "Submitting...";
  if (!to) return "Enter recipient";
  if (isEnsInput && isResolving) return "Resolving ENS...";
  if (isEnsInput && ensError) return "Invalid ENS name";
  if (!isEnsInput && !isAddress(to)) return "Invalid address";
  if (!Number.isFinite(amount) || amount <= 0) return "Enter amount";
  if (amount > balance) return "Insufficient balance";
  return `Send ${amount.toFixed(4)} IPT`;
}

function SummaryRow({
  label,
  children,
}: {
  label: string;
  children: ReactElement | string;
}): ReactElement {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-[#f0f0f0] py-2 last:border-b-0">
      <span className="text-[13px] text-[#6a6a6a]">{label}</span>
      <span className="min-w-0 text-right text-[13px] text-[#0a0a0a]">
        {children}
      </span>
    </div>
  );
}

export default function TransferPage(): ReactElement {
  const { isConnected, address } = useAccount();
  const { openConnectModal } = useConnectModal();
  const publicClient = usePublicClient();
  const { transact } = useTransactor();
  const { data: nativeBalance, refetch: refetchBalance } = useBalance({
    address,
  });

  const artBalance = nativeBalance
    ? Number(formatEther(nativeBalance.value))
    : 0;

  const [amount, setAmount] = useState("");
  const [to, setTo] = useState("");
  const [sending, setSending] = useState(false);
  const [success, setSuccess] = useState<SuccessState | null>(null);
  const [gasEstimate, setGasEstimate] = useState<string | null>(null);

  const { resolvedAddress, isResolving, ensError, isEnsInput } =
    useEnsResolve(to);

  const estimateTimerRef = useRef<number | null>(null);

  const numericAmount = parseDecimalAmount(amount);
  const validAmount = isValidDecimalAmount(amount) && numericAmount > 0;
  const amountError =
    amount && !isValidDecimalAmount(amount)
      ? "Enter a decimal amount using digits and one optional dot."
      : amount && numericAmount <= 0
        ? "Enter an amount greater than 0."
        : validAmount && numericAmount > artBalance
          ? "Insufficient balance."
          : "";

  const effectiveRecipient: `0x${string}` | null = isEnsInput
    ? resolvedAddress
    : isAddress(to)
      ? (to as `0x${string}`)
      : null;

  const recipientError =
    isEnsInput && !isResolving && ensError
      ? ensError
      : to && !isEnsInput && !isAddress(to)
        ? "Enter a valid address."
        : "";

  const valid =
    isConnected &&
    validAmount &&
    numericAmount <= artBalance &&
    effectiveRecipient !== null;

  const buttonLabel = getButtonLabel({
    amount: numericAmount,
    balance: artBalance,
    connected: isConnected,
    sending,
    to,
    isEnsInput,
    isResolving,
    ensError,
  });

  useEffect(() => {
    if (estimateTimerRef.current) {
      window.clearTimeout(estimateTimerRef.current);
      estimateTimerRef.current = null;
    }

    if (!valid || !publicClient || !address || !effectiveRecipient) {
      setGasEstimate(null);
      return;
    }

    estimateTimerRef.current = window.setTimeout(async () => {
      try {
        const gas = await publicClient.estimateGas({
          account: address,
          to: effectiveRecipient,
          value: parseEther(amount),
        });
        const gasPrice = await publicClient.getGasPrice();
        const fee = gas * gasPrice;
        setGasEstimate(`${formatEther(fee)} IPT`);
      } catch {
        setGasEstimate(null);
      }
    }, 500);

    return () => {
      if (estimateTimerRef.current) {
        window.clearTimeout(estimateTimerRef.current);
      }
    };
  }, [valid, effectiveRecipient, amount, publicClient, address]);

  async function handleSend() {
    if (!isConnected) {
      openConnectModal?.();
      return;
    }

    if (!valid || !effectiveRecipient) return;

    setSending(true);
    try {
      const hash = await transact({
        to: effectiveRecipient,
        value: parseEther(amount),
      });
      if (hash) {
        setSuccess({ amount: numericAmount, token: "IPT", hash });
        setAmount("");
        setTo("");
        refetchBalance();
      }
    } catch {
      // useTransactor handles error toasts
    } finally {
      setSending(false);
    }
  }

  function handleMax() {
    setAmount(String(artBalance));
    setSuccess(null);
  }

  return (
    <PageShell
      eyebrow="Transfer"
      title={
        <>
          Send IPT tokens.{" "}
          <span className="text-[#6a6a6a]">Deterministic finality.</span>
        </>
      }
      description="Transfer IPT across the Impetus network with deterministic GRANDPA finality."
    >
      <div className="grid w-full gap-6 lg:grid-cols-[1.4fr_1fr]">
        <section className="min-w-0">
          <DappPanel className="p-4 sm:p-6">
            <div className="rounded-3xl bg-[#f5f0e0] p-4 sm:p-5">
              <div className="flex items-center justify-between gap-4">
                <label
                  className="art-caption text-[#6a6a6a]"
                  htmlFor={amountInputId}
                >
                  You send
                </label>
                <div className="flex min-w-0 items-center gap-2 text-sm font-medium text-[#6a6a6a]">
                  <span className="truncate">
                    Balance{" "}
                    <Mono>
                      {formatTokenAmount(artBalance)} IPT
                    </Mono>
                  </span>
                  <button
                    className="min-h-8 shrink-0 rounded-full border border-[#0a0a0a]/10 bg-white px-4 text-xs font-black uppercase tracking-[0.12em] transition hover:border-[#0a0a0a]/25 disabled:cursor-not-allowed disabled:opacity-55 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#1a3a3a]"
                    disabled={sending}
                    onClick={handleMax}
                    type="button"
                  >
                    MAX
                  </button>
                </div>
              </div>

              <div className="mt-5 flex flex-col gap-3 sm:flex-row sm:items-center">
                <input
                  aria-describedby={amountError ? amountErrorId : undefined}
                  aria-invalid={!!amountError}
                  className="min-w-0 flex-1 bg-transparent text-5xl font-medium leading-none outline-none placeholder:text-[#0a0a0a]/25 sm:text-6xl"
                  disabled={sending}
                  id={amountInputId}
                  inputMode="decimal"
                  onChange={(event) => {
                    setAmount(event.target.value);
                    setSuccess(null);
                  }}
                  placeholder="0.00"
                  type="text"
                  value={amount}
                />
                <span className="inline-flex shrink-0 items-center gap-2 rounded-full border border-[#0a0a0a]/10 bg-white px-4 py-2">
                  <span
                    className="inline-flex size-6 shrink-0 items-center justify-center rounded-full border border-[#0a0a0a]/10 text-[9px] font-black shadow-[inset_0_1px_0_rgba(255,255,255,0.35)]"
                    style={{ backgroundColor: "#ffb084" }}
                  >
                    IP
                  </span>
                  <span className="text-sm font-semibold">IPT</span>
                </span>
              </div>

              {amountError && (
                <p
                  className="mt-3 text-sm font-black text-[#8f1d14]"
                  id={amountErrorId}
                >
                  {amountError}
                </p>
              )}
            </div>

            <label className="mt-5 block" htmlFor={recipientInputId}>
              <span className="art-caption text-[#6a6a6a]">To address</span>
              <input
                aria-describedby={
                  recipientError ? recipientErrorId : undefined
                }
                aria-invalid={!!recipientError}
                className="mt-2 h-14 w-full rounded-2xl border border-transparent bg-[#f5f0e0] px-5 font-mono text-sm text-[#0a0a0a] outline-none transition placeholder:text-[#0a0a0a]/30 focus:border-[#1a3a3a]"
                disabled={sending}
                id={recipientInputId}
                onChange={(event) => {
                  setTo(event.target.value);
                  setSuccess(null);
                }}
                placeholder="0x... or ENS name"
                spellCheck={false}
                type="text"
                value={to}
              />
            </label>
            {isEnsInput && isResolving && (
              <p className="mt-2 text-sm text-[#6a6a6a]">
                Resolving ENS name...
              </p>
            )}
            {isEnsInput && resolvedAddress && !isResolving && (
              <p className="mt-2 font-mono text-sm text-[#6a6a6a]">
                {shortHash(resolvedAddress)}
              </p>
            )}
            {recipientError && (
              <p
                className="mt-2 text-sm font-black text-[#8f1d14]"
                id={recipientErrorId}
              >
                {recipientError}
              </p>
            )}

            <div className="mt-5 rounded-md bg-[#faf5e8] p-4">
              <SummaryRow label="Network">
                <span className="inline-flex items-center gap-1.5">
                  <span className="size-1.5 rounded-full bg-[#22c55e]" />
                  Impetus (Chain ID 388266)
                </span>
              </SummaryRow>
              <SummaryRow label="Network fee">
                <Mono>{gasEstimate ?? "--"}</Mono>
              </SummaryRow>
              <SummaryRow label="Estimated finality">
                <Mono>~12s</Mono>
              </SummaryRow>
              <SummaryRow label="You send">
                <Mono>
                  {validAmount ? formatTokenAmount(numericAmount) : "0.00"}{" "}
                  IPT
                </Mono>
              </SummaryRow>
            </div>

            <button
              className="mt-5 min-h-12 w-full rounded-md bg-[#0a0a0a] px-6 text-sm font-semibold text-white transition hover:bg-[#1f1f1f] disabled:cursor-not-allowed disabled:bg-[#e5e5e5] disabled:text-[#6a6a6a]"
              disabled={sending || (isConnected ? !valid : !openConnectModal)}
              onClick={handleSend}
              type="button"
            >
              {buttonLabel}
            </button>

            {success && (
              <div
                aria-live="polite"
                className="mt-4 flex items-center justify-between gap-3 rounded-md bg-[#a4d4c5] p-3.5"
                role="status"
              >
                <span className="flex items-center gap-2.5 text-sm font-semibold text-[#0a0a0a]">
                  Sent {formatTokenAmount(success.amount)} {success.token}
                </span>
                <Mono className="text-xs">{shortHash(success.hash)}</Mono>
              </div>
            )}
          </DappPanel>
        </section>

        <aside className="flex min-w-0 flex-col gap-4">
          <div className="rounded-3xl bg-[#ffb084] p-6">
            <p className="art-caption opacity-70">IPT Balance</p>
            <h3 className="art-display mt-3 text-[2.75rem] leading-none tracking-tight">
              {formatTokenAmount(artBalance)}
            </h3>
            <p className="mt-2.5 text-sm font-semibold opacity-70">
              Impetus Token
            </p>
          </div>

          <div className="rounded-3xl border border-[#e5e5e5] bg-[#fffaf0] p-5">
            <p className="art-caption text-[#6a6a6a]">Network info</p>
            <div className="mt-3.5 flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <span className="text-[13px] text-[#6a6a6a]">Token</span>
                <span className="text-[13px] font-semibold">IPT (18 decimals)</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[13px] text-[#6a6a6a]">Chain ID</span>
                <Mono className="text-[13px]">388266</Mono>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[13px] text-[#6a6a6a]">Consensus</span>
                <span className="text-[13px] font-semibold">BABE + GRANDPA</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[13px] text-[#6a6a6a]">Finality</span>
                <span className="text-[13px] font-semibold">~6s</span>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </PageShell>
  );
}
