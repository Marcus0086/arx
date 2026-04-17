"use client";

import { useState, useRef, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Archive, Lock, MoreHorizontal, Pencil, ShieldCheck, Trash2 } from "lucide-react";
import type { Vault } from "@/src/sdk";
import { formatDistanceToNow } from "date-fns";

interface VaultCardProps {
  vault: Vault;
  onDelete: () => void;
}

function formatBytes(n: bigint | number) {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

export function VaultCard({ vault, onDelete }: VaultCardProps) {
  const router = useRouter();
  const sdk = useSdk();
  const qc = useQueryClient();

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(vault.name);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  const renameMutation = useMutation({
    mutationFn: (newName: string) => sdk.vaults.rename(vault.id, newName),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["vaults"] }),
  });

  function commitRename() {
    const trimmed = draft.trim();
    setEditing(false);
    if (trimmed && trimmed !== vault.name) {
      renameMutation.mutate(trimmed);
    } else {
      setDraft(vault.name);
    }
  }

  const createdDate = (() => {
    if (!vault.createdAt) return null;
    const d = new Date(vault.createdAt);
    return isNaN(d.getTime()) ? null : d;
  })();

  // Compression bar: how much of logical is stored
  const stats = vault.stats;
  const logical = stats ? Number(stats.logicalBytes) : 0;
  const stored = stats ? Number(stats.storedBytes) : Number(vault.sizeBytes);
  const savings = stats ? Number(stats.savingsBytes) : 0;
  const savedPct = logical > 0 ? Math.max(0, Math.round((savings / logical) * 100)) : 0;
  const storedPct =
    logical > 0 ? Math.min(100, Math.round((stored / logical) * 100)) : 100;

  return (
    <div
      className="group relative flex flex-col bg-pop rounded-lg p-1.5 gap-1 cursor-pointer ring-1 ring-transparent hover:ring-border transition-all"
      onClick={() => !editing && router.push(`/vaults/${vault.id}`)}
    >
      {/* Header: icon + menu */}
      <div className="flex items-center justify-between h-9 px-1 pr-1.5">
        <div className="flex items-center gap-2">
          <div className="flex items-center justify-center size-7 rounded bg-primary">
            <Archive className="size-4 opacity-80" />
          </div>
          <span className="text-xs uppercase text-muted-foreground tracking-wide">
            Vault
          </span>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
            <Button
              variant="ghost"
              size="icon-sm"
              className="size-7 opacity-0 group-hover:opacity-100 transition-opacity"
            >
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem
              className="gap-2"
              onClick={(e) => {
                e.stopPropagation();
                setDraft(vault.name);
                setEditing(true);
              }}
            >
              <Pencil className="w-3.5 h-3.5" />
              Rename
            </DropdownMenuItem>
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

      {/* Content */}
      <div className="flex flex-col gap-3 p-3 py-3 rounded bg-card flex-1">
        <div className="flex flex-col gap-1 min-h-[60px]">
          {editing ? (
            <input
              ref={inputRef}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onClick={(e) => e.stopPropagation()}
              onBlur={commitRename}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") {
                  setDraft(vault.name);
                  setEditing(false);
                }
              }}
              className="font-display text-xl uppercase leading-tight bg-transparent border-b border-primary outline-none"
              maxLength={128}
            />
          ) : (
            <p className="font-display text-xl uppercase leading-tight truncate">
              {vault.name}
            </p>
          )}
          <p className="text-xs text-muted-foreground uppercase tracking-wide">
            {formatBytes(stored)}
            {stats && logical > 0 && <> / {formatBytes(logical)} logical</>}
          </p>
        </div>

        {/* Compression bar */}
        {stats && logical > 0 && (
          <div className="space-y-0.5">
            <div className="h-1.5 rounded-sm bg-muted/40 overflow-hidden flex">
              <div className="h-full bg-primary" style={{ width: `${storedPct}%` }} />
              <div
                className="h-full bg-success/60"
                style={{ width: `${100 - storedPct}%` }}
              />
            </div>
            {savedPct > 0 && (
              <p className="text-[10px] uppercase text-success tracking-wide">
                Saved {savedPct}% ({formatBytes(savings)})
              </p>
            )}
          </div>
        )}

        <div className="flex items-center justify-between">
          {vault.encrypted ? (
            <Badge variant="outline-warning">
              <Lock />
              Encrypted
            </Badge>
          ) : (
            <Badge variant="outline-success">
              <ShieldCheck />
              Open
            </Badge>
          )}
          {createdDate && (
            <span className="text-[10px] text-muted-foreground uppercase">
              {formatDistanceToNow(createdDate, { addSuffix: true })}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
