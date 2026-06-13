"use client";

import { type ReactElement, useState } from "react";
import { Button } from "@artemis/coss-ui/ui/button";

interface CodeBlockProps {
  code: string;
  label?: string;
}

export function CodeBlock({ code, label }: CodeBlockProps): ReactElement {
  const [copied, setCopied] = useState(false);

  async function copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard unavailable (insecure context) — selection still works.
    }
  }

  return (
    <div className="overflow-hidden rounded-2xl border border-[#1f3a3a] bg-[#1a3a3a]">
      <div className="flex items-center justify-between border-b border-white/10 px-4 py-2">
        <span className="font-mono text-[11px] uppercase tracking-[0.1em] text-white/55">
          {label ?? "shell"}
        </span>
        <Button type="button" variant="outline" size="sm" onClick={copy}>
          {copied ? "Copied" : "Copy"}
        </Button>
      </div>
      <pre className="overflow-x-auto px-4 py-3.5">
        <code className="font-mono text-[12.5px] leading-relaxed text-[#e9efe9]">{code}</code>
      </pre>
    </div>
  );
}
