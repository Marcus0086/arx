"use client";

import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Archive, Lock, MoreHorizontal, ShieldCheck, Trash2 } from "lucide-react";
import type { Vault } from "@/src/sdk";
import { formatDistanceToNow } from "date-fns";

interface VaultCardProps {
  vault: Vault;
  onDelete: () => void;
}

export function VaultCard({ vault, onDelete }: VaultCardProps) {
  const router = useRouter();

  const createdDate = vault.createdAt ? new Date(vault.createdAt) : null;

  return (
    <div
      className="group relative flex flex-col bg-card border border-border/50 rounded-xl p-4 gap-3 hover:border-border transition-all cursor-pointer hover:shadow-lg hover:shadow-black/5"
      onClick={() => router.push(`/vaults/${vault.id}`)}
    >
      {/* Icon + menu */}
      <div className="flex items-start justify-between">
        <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-primary/10 border border-primary/20">
          <Archive className="w-5 h-5 text-primary" />
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity"
            >
              <MoreHorizontal className="w-4 h-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem
              className="text-destructive focus:text-destructive gap-2"
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
            >
              <Trash2 className="w-3.5 h-3.5" />
              Delete vault
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Vault info */}
      <div className="space-y-1 flex-1">
        <p className="font-semibold text-sm leading-tight truncate">{vault.name}</p>
        <p className="text-xs text-muted-foreground">{formatBytes(vault.sizeBytes)}</p>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-1 border-t border-border/30">
        <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
          {vault.encrypted ? (
            <>
              <Lock className="w-3 h-3 text-yellow-500" />
              <span className="text-yellow-600 dark:text-yellow-400">Encrypted</span>
            </>
          ) : (
            <>
              <ShieldCheck className="w-3 h-3 text-green-500" />
              <span>Open</span>
            </>
          )}
        </div>
        {createdDate && (
          <span className="text-[10px] text-muted-foreground">
            {formatDistanceToNow(createdDate, { addSuffix: true })}
          </span>
        )}
      </div>
    </div>
  );
}

function formatBytes(n: bigint | number) {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}
