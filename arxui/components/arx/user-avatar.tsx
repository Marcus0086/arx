"use client";

import Image from "next/image";
import { cn } from "@/lib/utils";

const AVATARS = [
  "/avatars/user_krimson.png",
  "/avatars/user_mati.png",
  "/avatars/user_pek.png",
  "/avatars/user_joyboy.png",
];

/**
 * Deterministic hash → used to pick avatar + filter so a given user always
 * lands on the same variant across reloads (no flicker).
 */
function hashString(s: string): number {
  let h = 2166136261 >>> 0; // FNV-1a
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

type FilterStyle = {
  filter: string;
  mix?: string;
};

const FILTERS: FilterStyle[] = [
  { filter: "none" },
  { filter: "grayscale(100%)" },
  { filter: "saturate(180%) hue-rotate(120deg)" },
  { filter: "saturate(180%) hue-rotate(210deg)" },
  { filter: "saturate(180%) hue-rotate(300deg)" },
  { filter: "sepia(60%) saturate(200%)" },
  { filter: "invert(10%) hue-rotate(60deg) saturate(180%)" },
];

interface Props {
  seed: string;
  size?: number;
  className?: string;
  rounded?: boolean;
}

export function UserAvatar({ seed, size = 56, className, rounded = true }: Props) {
  const h = hashString(seed || "anon");
  const src = AVATARS[h % AVATARS.length];
  const { filter } = FILTERS[(h >>> 3) % FILTERS.length];

  return (
    <div
      className={cn(
        "relative overflow-clip bg-sidebar-primary text-sidebar-primary-foreground shrink-0",
        rounded ? "rounded-lg" : "rounded",
        className,
      )}
      style={{ width: size, height: size }}
    >
      <Image
        src={src}
        alt={seed}
        width={size * 2}
        height={size * 2}
        className="w-full h-full object-cover"
        style={{ filter }}
        priority={false}
        unoptimized
      />
    </div>
  );
}
