"use client";

import { Bullet } from "@/components/ui/bullet";
import type { VaultStats } from "@/src/sdk";

interface Props {
  stats: VaultStats;
}

function formatBytes(n: bigint | number) {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

export function VaultStatsStrip({ stats }: Props) {
  const logical = Number(stats.logicalBytes);
  const stored = Number(stats.storedBytes);
  const saved = Number(stats.savingsBytes);
  const savedPct = logical > 0 ? Math.max(0, (saved / logical) * 100) : 0;
  const storedPct = logical > 0 ? Math.min(100, (stored / logical) * 100) : 100;

  return (
    <div className="bg-pop rounded-lg p-1.5 flex flex-col gap-1">
      <div className="grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 pl-1 pr-1.5 py-2">
        <div className="flex items-center gap-2 text-xs uppercase text-muted-foreground">
          <Bullet variant="success" />
          Storage · Compression
        </div>
      </div>

      <div className="p-3 rounded bg-card space-y-4">
        {/* Stacked compression bar */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-[10px] uppercase tracking-wide">
            <span className="text-muted-foreground">Stored</span>
            <span className="text-success">
              {savedPct > 0 ? `Saved ${savedPct.toFixed(0)}%` : "No compression"}
            </span>
          </div>
          <div className="h-3 rounded-sm overflow-hidden flex bg-muted/40">
            <div className="h-full bg-primary" style={{ width: `${storedPct}%` }} />
            <div
              className="h-full bg-success/60"
              style={{ width: `${100 - storedPct}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-[10px] uppercase tracking-wide text-muted-foreground">
            <span>{formatBytes(stored)} stored</span>
            <span>{formatBytes(logical)} logical</span>
          </div>
        </div>

        {/* Stat tiles */}
        <div className="grid grid-cols-4 gap-3 pt-1 border-t border-border/40">
          <StatTile label="Files" value={stats.files} />
          <StatTile label="Chunks" value={stats.chunks} />
          <StatTile label="Logical" value={formatBytes(stats.logicalBytes)} />
          <StatTile label="Stored" value={formatBytes(stats.storedBytes)} />
        </div>
      </div>
    </div>
  );
}

function StatTile({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex flex-col gap-0.5 min-w-0">
      <span className="text-[10px] uppercase text-muted-foreground tracking-wide">
        {label}
      </span>
      <span className="font-display text-xl truncate">{value}</span>
    </div>
  );
}
