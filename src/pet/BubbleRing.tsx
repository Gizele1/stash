import type { BubbleState, AnchorPosition } from "./types";
import { Bubble } from "./Bubble";

interface BubbleRingProps {
  bubbles: BubbleState[];
  /** Radius of the ring from center (px) */
  radius?: number;
  /** Callback when a bubble is clicked */
  onBubbleClick: (contextId: string, anchorPosition: AnchorPosition) => void;
}

/** Default ring radius in pixels */
const DEFAULT_RING_RADIUS = 56;

/**
 * BubbleRing renders 0-4 context status bubbles around the pet sprite.
 * Bubbles are positioned at clock-position-based angles.
 */
export function BubbleRing({
  bubbles,
  radius = DEFAULT_RING_RADIUS,
  onBubbleClick,
}: BubbleRingProps) {
  if (bubbles.length === 0) return null;

  return (
    <div className="bubble-ring">
      {bubbles.map((bubble) => (
        <Bubble
          key={bubble.contextId}
          state={bubble}
          radius={radius}
          onClick={onBubbleClick}
        />
      ))}
    </div>
  );
}
