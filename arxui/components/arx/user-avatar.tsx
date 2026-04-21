import { cn } from "@/lib/utils";

const AVATARS = [
  "/avatars/user_krimson.png",
  "/avatars/user_mati.png",
  "/avatars/user_pek.png",
  "/avatars/user_joyboy.png",
];

function hashString(s: string): number {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

const FILTERS: string[] = [
  "none",
  "grayscale(100%)",
  "saturate(180%) hue-rotate(120deg)",
  "saturate(180%) hue-rotate(210deg)",
  "saturate(180%) hue-rotate(300deg)",
  "sepia(60%) saturate(200%)",
  "invert(10%) hue-rotate(60deg) saturate(180%)",
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
  const filter = FILTERS[(h >>> 3) % FILTERS.length];

  return (
    <div
      className={cn(
        "relative overflow-clip bg-sidebar-primary text-sidebar-primary-foreground shrink-0",
        rounded ? "rounded-lg" : "rounded",
        className,
      )}
      style={{ width: size, height: size }}
    >
      <img
        src={src}
        alt={seed}
        width={size * 2}
        height={size * 2}
        className="w-full h-full object-cover"
        style={{ filter }}
      />
    </div>
  );
}
