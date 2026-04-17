"use client";

import { useMemo } from "react";
import { useQuery, useQueries } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Bullet } from "@/components/ui/bullet";
import { AnimatePresence, motion } from "framer-motion";
import { Clock, File } from "lucide-react";
import type { Vault } from "@/src/sdk";

interface ActivityItem {
  vault: Vault;
  path: string;
  size: bigint;
  mtime: bigint;
}

function formatBytes(n: bigint | number): string {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

function timeAgo(mtime: bigint): string {
  const secs = Number(mtime);
  if (!isFinite(secs) || secs <= 0) return "";
  const now = Date.now() / 1000;
  const diff = Math.max(0, now - secs);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function RecentActivityWidget() {
  const sdk = useSdk();
  const { data: vaults = [] } = useQuery({
    queryKey: ["vaults"],
    queryFn: () => sdk.vaults.list(),
  });

  const fileQueries = useQueries({
    queries: vaults.map((v) => ({
      queryKey: ["vault-files-recent", v.id],
      queryFn: () => sdk.files.list(v.id),
      staleTime: 30_000,
    })),
  });

  const items = useMemo<ActivityItem[]>(() => {
    const all: ActivityItem[] = [];
    vaults.forEach((vault, i) => {
      const q = fileQueries[i];
      if (!q?.data) return;
      for (const f of q.data.entries) {
        all.push({ vault, path: f.path, size: f.size, mtime: f.mtime });
      }
    });
    all.sort((a, b) => Number(b.mtime - a.mtime));
    return all.slice(0, 4);
  }, [vaults, fileQueries]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center">
          <Bullet className="mr-2" variant="warning" />
          <span className="uppercase">Recent</span>
          <Clock className="ml-auto size-4 opacity-60" />
        </CardTitle>
      </CardHeader>
      <CardContent className="p-3 rounded bg-card">
        <AnimatePresence mode="popLayout">
          {items.length === 0 && (
            <motion.div
              key="empty"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="text-xs uppercase text-muted-foreground py-4 text-center"
            >
              No recent activity
            </motion.div>
          )}
          {items.map((it) => (
            <motion.div
              key={`${it.vault.id}:${it.path}`}
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex items-center gap-2 py-2 border-b border-border/30 last:border-0"
            >
              <File className="size-4 shrink-0 text-muted-foreground" />
              <div className="flex-1 min-w-0">
                <div className="truncate text-xs">{it.path}</div>
                <div className="flex items-center gap-2 text-[10px] uppercase text-muted-foreground">
                  <span className="truncate">{it.vault.name}</span>
                  <span>·</span>
                  <span>{formatBytes(it.size)}</span>
                </div>
              </div>
              <span className="text-[10px] uppercase text-muted-foreground whitespace-nowrap">
                {timeAgo(it.mtime)}
              </span>
            </motion.div>
          ))}
        </AnimatePresence>
      </CardContent>
    </Card>
  );
}
