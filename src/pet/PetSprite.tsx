import { useEffect, useRef, useState } from "react";
import type { SpriteConfig } from "./types";
import { DEFAULT_SPRITE_CONFIG } from "./types";

interface PetSpriteProps {
  config?: SpriteConfig;
}

/**
 * PetSprite renders a 64x64 pixel art sprite with idle animation.
 * Uses requestAnimationFrame with frame skipping to render at 4fps (not 60fps)
 * for <1% CPU usage.
 *
 * When no sprite sheet is provided, renders a placeholder colored square.
 */
export function PetSprite({ config = DEFAULT_SPRITE_CONFIG }: PetSpriteProps) {
  const [frameIndex, setFrameIndex] = useState(0);
  const lastFrameTimeRef = useRef(0);
  const rafRef = useRef<number>(0);

  const hasSheet = config.sheetUrl.length > 0;
  const frameInterval = 1000 / config.fps;

  useEffect(() => {
    if (!hasSheet) return;

    const animate = (timestamp: number) => {
      if (timestamp - lastFrameTimeRef.current >= frameInterval) {
        lastFrameTimeRef.current = timestamp;
        setFrameIndex((prev) => (prev + 1) % config.idleFrames.length);
      }
      rafRef.current = requestAnimationFrame(animate);
    };

    rafRef.current = requestAnimationFrame(animate);

    return () => {
      cancelAnimationFrame(rafRef.current);
    };
  }, [hasSheet, frameInterval, config.idleFrames.length]);

  if (!hasSheet) {
    return (
      <div className="pet-sprite">
        <div className="pet-sprite--placeholder pet-sprite--breathing" />
      </div>
    );
  }

  const currentFrame = config.idleFrames[frameIndex] ?? 0;
  const framesPerRow = Math.floor(1024 / config.frameWidth); // assume 1024px sheet width
  const sx = (currentFrame % framesPerRow) * config.frameWidth;
  const sy = Math.floor(currentFrame / framesPerRow) * config.frameHeight;

  return (
    <div className="pet-sprite">
      <div
        className="pet-sprite--sheet"
        style={{
          width: config.frameWidth,
          height: config.frameHeight,
        }}
      >
        <img
          src={config.sheetUrl}
          alt="Pet sprite"
          style={{
            transform: `translate(-${sx}px, -${sy}px)`,
          }}
          draggable={false}
        />
      </div>
    </div>
  );
}
