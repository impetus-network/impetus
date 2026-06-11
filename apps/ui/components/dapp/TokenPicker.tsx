"use client";

import { ChevronDown } from "lucide-react";
import {
  type KeyboardEvent,
  type ReactElement,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import type { DemoToken } from "./types";
import { TokenIcon } from "./DappPrimitives";

type TokenPickerProps = {
  tokens: DemoToken[];
  selectedToken: DemoToken;
  open: boolean;
  disabled?: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (token: DemoToken) => void;
};

export function TokenPicker({
  tokens,
  selectedToken,
  open,
  disabled = false,
  onOpenChange,
  onSelect,
}: TokenPickerProps): ReactElement {
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [focusedIndex, setFocusedIndex] = useState(0);

  const selectedIndex = tokens.findIndex(
    (token) => token.sym === selectedToken.sym,
  );

  function getOption(index: number): HTMLButtonElement | null {
    return (
      listRef.current?.querySelector<HTMLButtonElement>(
        `[data-option-index="${index}"]`,
      ) ?? null
    );
  }

  function getCurrentFocusIndex(): number {
    const activeElement = document.activeElement;

    if (!(activeElement instanceof HTMLElement)) {
      return Math.min(focusedIndex, tokens.length - 1);
    }

    const optionIndex = activeElement.dataset.optionIndex;

    if (!optionIndex) return Math.min(focusedIndex, tokens.length - 1);

    const parsedIndex = Number(optionIndex);

    return Number.isInteger(parsedIndex) && parsedIndex < tokens.length
      ? parsedIndex
      : Math.min(focusedIndex, tokens.length - 1);
  }

  function focusOption(index: number) {
    const option = getOption(index);

    if (!option) return;

    setFocusedIndex(index);
    option.focus();
  }

  function closeAndFocusTrigger() {
    onOpenChange(false);
    triggerRef.current?.focus();
  }

  useEffect(() => {
    if (disabled && open) {
      onOpenChange(false);
    }
  }, [disabled, onOpenChange, open]);

  useEffect(() => {
    if (!open) return;

    function handlePointerDown(event: PointerEvent) {
      const target = event.target;

      if (
        target instanceof Node &&
        rootRef.current &&
        !rootRef.current.contains(target)
      ) {
        onOpenChange(false);
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [onOpenChange, open]);

  useEffect(() => {
    if (!open || disabled || tokens.length === 0) return;

    const initialIndex = selectedIndex >= 0 ? selectedIndex : 0;

    focusOption(initialIndex);
  }, [disabled, open, selectedIndex, tokens.length]);

  function handleButtonKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "Escape") {
      closeAndFocusTrigger();
    }
  }

  function handleListKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (disabled || tokens.length === 0) return;

    const currentIndex = getCurrentFocusIndex();

    if (event.key === "Escape") {
      event.preventDefault();
      closeAndFocusTrigger();
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusOption((currentIndex + 1) % tokens.length);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      focusOption((currentIndex - 1 + tokens.length) % tokens.length);
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      focusOption(0);
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      focusOption(tokens.length - 1);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      handleSelect(tokens[currentIndex]);
    }
  }

  function handleSelect(token: DemoToken) {
    if (disabled) return;

    onSelect(token);
    onOpenChange(false);
  }

  return (
    <div className="relative" ref={rootRef}>
      <button
        aria-controls={open ? listId : undefined}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label="Select token"
        className="inline-flex h-12 items-center gap-2 rounded-full border border-[#0a0a0a]/10 bg-white px-3 text-sm font-black text-[#0a0a0a] shadow-[inset_0_1px_0_rgba(255,255,255,0.7)] transition hover:border-[#0a0a0a]/25 disabled:cursor-not-allowed disabled:opacity-55 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#1a3a3a]"
        disabled={disabled}
        id={`${listId}-button`}
        onClick={() => onOpenChange(!open)}
        onKeyDown={handleButtonKeyDown}
        ref={triggerRef}
        type="button"
      >
        <TokenIcon token={selectedToken} size={28} />
        <span>{selectedToken.sym}</span>
        <ChevronDown
          aria-hidden="true"
          className={
            open
              ? "size-4 rotate-180 text-[#6a6a6a]"
              : "size-4 text-[#6a6a6a]"
          }
        />
      </button>

      {open && (
        <div
          aria-labelledby={`${listId}-button`}
          className="absolute right-0 z-30 mt-2 w-64 overflow-hidden rounded-2xl border border-[#0a0a0a]/10 bg-white shadow-xl"
          id={listId}
          onKeyDown={handleListKeyDown}
          ref={listRef}
          role="listbox"
        >
          {tokens.map((token, index) => (
            <button
              aria-selected={token.sym === selectedToken.sym}
              data-option-index={index}
              disabled={disabled}
              className="flex w-full items-center justify-between gap-4 px-4 py-3 text-left transition hover:bg-[#f5f0e0] focus-visible:bg-[#f5f0e0] focus-visible:outline-none"
              key={token.sym}
              onClick={() => handleSelect(token)}
              onFocus={() => setFocusedIndex(index)}
              role="option"
              tabIndex={index === focusedIndex ? 0 : -1}
              type="button"
            >
              <span className="flex min-w-0 items-center gap-3">
                <TokenIcon token={token} size={30} />
                <span className="min-w-0">
                  <span className="block truncate text-sm font-black text-[#0a0a0a]">
                    {token.sym}
                  </span>
                  <span className="block truncate text-xs font-medium text-[#6a6a6a]">
                    {token.name}
                  </span>
                </span>
              </span>
              <span className="shrink-0 font-mono text-xs text-[#6a6a6a]">
                {token.balance.toLocaleString("en-US")}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
