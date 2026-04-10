import type { ContextWithStatus } from "../types";
import { STATUS_COLORS } from "../types";

interface StatusBadgeProps {
  status: ContextWithStatus["status"];
}

const LABELS: Record<ContextWithStatus["status"], string> = {
  running: "RUNNING",
  done: "DONE",
  stuck: "STUCK",
  parked: "PARKED",
};

export function StatusBadge({ status }: StatusBadgeProps) {
  const color = STATUS_COLORS[status];
  return (
    <span
      className="panels-status-badge"
      style={{
        color,
        borderColor: color,
      }}
      data-status={status}
    >
      {LABELS[status]}
    </span>
  );
}
