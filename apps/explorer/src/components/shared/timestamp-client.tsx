"use client";

import { useEffect, useState } from "react";
import { formatRelativeTime } from "@artemis/shared";

interface TimestampClientProps {
  unixSec: number;
}

export function TimestampClient({ unixSec }: TimestampClientProps) {
  const [text, setText] = useState(formatRelativeTime(unixSec));

  useEffect(() => {
    const interval = setInterval(() => {
      setText(formatRelativeTime(unixSec));
    }, 60_000);
    return () => clearInterval(interval);
  }, [unixSec]);

  return (
    <span title={new Date(unixSec * 1000).toISOString()}>{text}</span>
  );
}
