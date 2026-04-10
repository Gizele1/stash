import type { BubbleState, AnchorPosition } from "./types";

interface BubbleProps {
  state: BubbleState;
  /** Distance from center to place bubble */
  radius: number;
  /** Callback when bubble is clicked */
  onClick: (contextId: string, anchorPosition: AnchorPosition) => void;
}

/**
 * Bubble renders an individual context status bubble.
 * Positioned along the ring at the specified angle and radius.
 * Shows status color, optional pulse animation, and processing indicator.
 */
export function Bubble({ state, radius, onClick }: BubbleProps) {
  const { contextId, status, positionAngle, color, isPulsing, isProcessing } =
    state;

  // Calculate position from angle and radius
  // Angle 0 = 12 o'clock, increases clockwise
  const x = Math.sin(positionAngle) * radius;
  const y = -Math.cos(positionAngle) * radius;

  // Calculate connector line to center
  const connectorLength = Math.sqrt(x * x + y * y) - 14; // subtract bubble radius + gap
  const connectorAngle = Math.atan2(x, -y); // angle from bubble back to center

  const classNames = [
    "bubble",
    `bubble--${status}`,
    isPulsing ? "bubble--pulsing" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const handleClick = (e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    onClick(contextId, {
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
    });
  };

  return (
    <div
      className={classNames}
      style={{
        transform: `translate(calc(-50% + ${x}px), calc(-50% + ${y}px))`,
      }}
      onClick={handleClick}
      data-context-id={contextId}
    >
      {/* Connector line to pet */}
      {connectorLength > 0 && (
        <div
          className="bubble__connector"
          style={{
            height: connectorLength,
            transform: `rotate(${connectorAngle + Math.PI}rad)`,
            top: "50%",
            left: "50%",
          }}
        />
      )}

      {/* Status circle */}
      <div
        className="bubble__circle"
        style={{ backgroundColor: color }}
      >
        {/* Processing spinner */}
        {isProcessing && <div className="bubble__processing" />}
      </div>
    </div>
  );
}
