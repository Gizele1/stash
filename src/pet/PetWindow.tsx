import { useRef, useCallback, useState } from "react";
import { PetSprite } from "./PetSprite";
import { BubbleRing } from "./BubbleRing";
import { usePetState } from "./usePetState";
import type { SpriteConfig } from "./types";
import "./pet.css";

interface PetWindowProps {
  spriteConfig?: SpriteConfig;
}

/**
 * PetWindow is the main component for the transparent pet overlay.
 * Orchestrates the pixel sprite and context status bubbles.
 * Supports dragging the pet to reposition it on screen.
 */
export function PetWindow({ spriteConfig }: PetWindowProps) {
  const { bubbles, handleBubbleClick, handleDragEnd } = usePetState();
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef<{ x: number; y: number } | null>(null);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      // Only start drag on the pet area, not on bubbles
      const target = e.target as HTMLElement;
      if (target.closest(".bubble")) return;

      setIsDragging(true);
      dragStartRef.current = { x: e.clientX, y: e.clientY };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    []
  );

  const handlePointerMove = useCallback(
    () => {
      if (!isDragging) return;
      // During drag, the Tauri window itself moves.
      // Actual window movement is handled by Tauri's drag API.
      // This is a visual feedback placeholder.
    },
    [isDragging]
  );

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (!isDragging) return;
      setIsDragging(false);

      // Report final position
      handleDragEnd(e.screenX, e.screenY);
      dragStartRef.current = null;
    },
    [isDragging, handleDragEnd]
  );

  const containerClass = [
    "pet-window-container",
    isDragging ? "dragging" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      ref={containerRef}
      className={containerClass}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      <PetSprite config={spriteConfig} />
      <BubbleRing bubbles={bubbles} onBubbleClick={handleBubbleClick} />
    </div>
  );
}
