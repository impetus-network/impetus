"use client";

import { useState } from "react";

interface JsonViewerProps {
  data: unknown;
  label?: string;
}

export function JsonViewer({ data, label = "JSON" }: JsonViewerProps) {
  const [expanded, setExpanded] = useState(false);

  if (!data) return <span className="text-gray-400">null</span>;

  const json = typeof data === "string" ? data : JSON.stringify(data, null, 2);

  return (
    <div>
      <button
        onClick={() => setExpanded((prev) => !prev)}
        className="text-sm text-blue-600 hover:underline"
      >
        {expanded ? `Hide ${label}` : `Show ${label}`}
      </button>
      {expanded && (
        <pre className="mt-2 max-h-96 overflow-auto rounded-lg bg-gray-50 p-4 text-xs font-mono">
          {json}
        </pre>
      )}
    </div>
  );
}
