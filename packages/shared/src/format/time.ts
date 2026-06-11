import { formatDistanceToNow } from "date-fns";

export function formatRelativeTime(unixSec: number): string {
  return formatDistanceToNow(new Date(unixSec * 1000), { addSuffix: true });
}

export function formatTimestamp(unixSec: number): string {
  const fmt = new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "medium",
    timeZone: "UTC",
  });
  return `${fmt.format(new Date(unixSec * 1000))} UTC`;
}
