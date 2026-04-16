"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { VaultCard } from "@/components/arx/vault-card";
import { CreateVaultDialog } from "@/components/arx/create-vault-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Plus, Search, HardDrive } from "lucide-react";
import type { Vault } from "@/src/sdk";

export default function VaultsPage() {
  const sdk = useSdk();
  const qc = useQueryClient();
  const [search, setSearch] = useState("");
  const [createOpen, setCreateOpen] = useState(false);

  const { data: vaults = [], isLoading } = useQuery({
    queryKey: ["vaults"],
    queryFn: () => sdk.vaults.list(),
  });

  const deleteVault = useMutation({
    mutationFn: (id: string) => sdk.vaults.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["vaults"] }),
  });

  const filtered = vaults.filter((v) =>
    v.name.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="p-6 max-w-6xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <HardDrive className="w-6 h-6 text-primary" />
            My Vaults
          </h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            {vaults.length} vault{vaults.length !== 1 ? "s" : ""} · encrypted archive storage
          </p>
        </div>
        <Button onClick={() => setCreateOpen(true)} className="gap-2">
          <Plus className="w-4 h-4" />
          New Vault
        </Button>
      </div>

      {/* Search */}
      {vaults.length > 4 && (
        <div className="relative max-w-xs">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            placeholder="Search vaults…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
      )}

      {/* Vault Grid */}
      {isLoading ? (
        <VaultGridSkeleton />
      ) : filtered.length === 0 ? (
        <EmptyState onCreate={() => setCreateOpen(true)} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {filtered.map((vault) => (
            <VaultCard
              key={vault.id}
              vault={vault}
              onDelete={() => deleteVault.mutate(vault.id)}
            />
          ))}
        </div>
      )}

      <CreateVaultDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onCreated={() => qc.invalidateQueries({ queryKey: ["vaults"] })}
      />
    </div>
  );
}

function VaultGridSkeleton() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="h-44 rounded-xl bg-muted/40 animate-pulse" />
      ))}
    </div>
  );
}

function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center space-y-4">
      <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-muted/50 border border-border/50">
        <HardDrive className="w-8 h-8 text-muted-foreground/50" />
      </div>
      <div>
        <p className="font-medium">No vaults yet</p>
        <p className="text-sm text-muted-foreground mt-1">
          Create your first vault to start storing encrypted files
        </p>
      </div>
      <Button onClick={onCreate} variant="outline" className="gap-2">
        <Plus className="w-4 h-4" />
        Create a vault
      </Button>
    </div>
  );
}
