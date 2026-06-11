"use client";

import { useState, useCallback } from "react";

interface TxInputDataProps {
  data: string;
}

const PREVIEW_LENGTH = 66;

export function TxInputData({ data }: TxInputDataProps) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(data);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [data]);

  if (!data || data === "0x") {
    return <span className="text-gray-400">--</span>;
  }

  const isLong = data.length > PREVIEW_LENGTH;
  const display = expanded || !isLong ? data : `${data.slice(0, PREVIEW_LENGTH)}...`;

  return (
    <div>
      <div className="max-h-48 overflow-y-auto rounded border border-gray-200 bg-gray-50 p-2">
        <code className="font-mono text-xs break-all">{display}</code>
      </div>
      <div className="mt-1 flex gap-3">
        {isLong && (
          <button
            onClick={() => setExpanded((prev) => !prev)}
            className="text-xs text-blue-600 hover:underline"
          >
            {expanded ? "Collapse" : "Expand"}
          </button>
        )}
        <button
          onClick={handleCopy}
          className="text-xs text-gray-400 hover:text-gray-600 transition-colors"
        >
          {copied ? "Copied!" : "Copy"}
        </button>
      </div>
    </div>
  );
}
