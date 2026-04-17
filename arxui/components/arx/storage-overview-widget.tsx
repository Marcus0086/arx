"use client";

import { useQuery } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Bullet } from "@/components/ui/bullet";
import { HardDrive } from "lucide-react";
import { cn } from "@/lib/utils";

function formatBytes(n: bigint | number): string {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

export function StorageOverviewWidget() {
  const sdk = useSdk();
  const { data: vaults = [] } = useQuery({
    queryKey: ["vaults"],
    queryFn: () => sdk.vaults.list(),
  });

  let totalLogical = 0n;
  let totalStored = 0n;
  let totalFiles = 0;
  for (const v of vaults) {
    if (v.stats) {
      totalLogical += v.stats.logicalBytes;
      totalStored += v.stats.storedBytes;
      totalFiles += v.stats.files;
    }
  }
  const saved = totalLogical > totalStored ? totalLogical - totalStored : 0n;
  const savedPct = totalLogical > 0n ? Number((saved * 100n) / totalLogical) : 0;
  const storedPct =
    totalLogical > 0n ? Math.min(100, Number((totalStored * 100n) / totalLogical)) : 0;

  // Arrow direction: down (positive) when we're saving a meaningful amount,
  // up (negative) when overhead exceeds logical, otherwise no marquee.
  const direction: "up" | "down" | null =
    savedPct >= 10 ? "down" : totalLogical > 0n && savedPct <= -5 ? "up" : null;
  const intent: "positive" | "negative" = direction === "down" ? "positive" : "negative";

  return (
    <Card className="relative overflow-hidden">
      <CardHeader>
        <CardTitle className="flex items-center">
          <Bullet className="mr-2" variant="success" />
          <span className="uppercase">Storage</span>
          <HardDrive className="ml-auto size-4 opacity-60" />
        </CardTitle>
      </CardHeader>
      <CardContent className="bg-accent/30 flex-1 pt-3 overflow-clip relative z-10">
        <div className="flex items-baseline">
          <span className="text-4xl md:text-5xl font-display">
            {formatBytes(totalStored)}
          </span>
        </div>
        <p className="text-xs uppercase text-muted-foreground tracking-wide mt-1">
          Stored across {vaults.length} vault{vaults.length !== 1 ? "s" : ""}
        </p>

        {/* Usage bar */}
        <div className="mt-3 space-y-1">
          <div className="h-2 rounded-sm bg-muted/40 overflow-hidden flex">
            <div className="h-full bg-primary" style={{ width: `${storedPct}%` }} />
            <div
              className="h-full bg-success/50"
              style={{ width: `${Math.max(0, 100 - storedPct)}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-[10px] uppercase tracking-wide">
            <span className="text-muted-foreground">
              {formatBytes(totalLogical)} logical
            </span>
            <span className={savedPct > 0 ? "text-success" : "text-muted-foreground"}>
              Saved {formatBytes(saved)} ({savedPct}%)
            </span>
          </div>
        </div>

        {/* Bottom stats row */}
        <div className="flex items-center justify-between text-xs uppercase border-t border-border/40 pt-2 mt-3">
          <div className="flex flex-col">
            <span className="opacity-50">Vaults</span>
            <span className="font-display text-xl">{vaults.length}</span>
          </div>
          <div className="flex flex-col items-end">
            <span className="opacity-50">Files</span>
            <span className="font-display text-xl">{totalFiles}</span>
          </div>
        </div>

        {/* Marquee Animation */}
        {direction && (
          <div className="absolute top-0 right-0 w-14 h-full pointer-events-none overflow-hidden group">
            <div
              className={cn(
                "flex flex-col transition-all duration-500",
                "group-hover:scale-105 group-hover:brightness-110",
                intent === "positive" ? "text-success" : "text-destructive",
                direction === "up" ? "animate-marquee-up" : "animate-marquee-down",
              )}
            >
              <div
                className={cn(
                  "flex",
                  direction === "up" ? "flex-col-reverse" : "flex-col",
                )}
              >
                {Array.from({ length: 6 }).map((_, i) => (
                  <Arrow key={`a-${i}`} direction={direction} index={i} />
                ))}
              </div>
              <div
                className={cn(
                  "flex",
                  direction === "up" ? "flex-col-reverse" : "flex-col",
                )}
              >
                {Array.from({ length: 6 }).map((_, i) => (
                  <Arrow key={`b-${i}`} direction={direction} index={i} />
                ))}
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function Arrow({ direction, index }: { direction: "up" | "down"; index: number }) {
  const staggerDelay = index * 0.15;
  const phaseDelay = (index % 3) * 0.8;
  return (
    <span
      style={{
        animationDelay: `${staggerDelay + phaseDelay}s`,
        animationDuration: "3s",
        animationTimingFunction: "cubic-bezier(0.4, 0.0, 0.2, 1)",
      }}
      className={cn(
        "text-center text-5xl size-14 font-display leading-none block",
        "transition-all duration-700 ease-out",
        "animate-marquee-pulse will-change-transform",
      )}
    >
      {direction === "up" ? "↑" : "↓"}
    </span>
  );
}
