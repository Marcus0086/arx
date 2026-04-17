"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { VaultCard } from "@/components/arx/vault-card";
import { CreateVaultDialog } from "@/components/arx/create-vault-dialog";
import { DashboardPageLayout } from "@/components/arx/dashboard-page-layout";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Plus, Search, HardDrive } from "lucide-react";

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

  const count = vaults.length;
  const description = `${count} vault${count !== 1 ? "s" : ""} · ENCRYPTED`;

  return (
    <DashboardPageLayout header={{ title: "MY VAULTS", icon: HardDrive, description }}>
      {/* Toolbar */}
      <div className="flex items-center gap-3 justify-between">
        {vaults.length > 4 ? (
          <div className="relative max-w-xs flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              placeholder="Search vaults…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        ) : (
          <div />
        )}
        <Button onClick={() => setCreateOpen(true)}>
          <Plus />
          New Vault
        </Button>
      </div>

      {/* Vault Grid */}
      {isLoading ? (
        <VaultGridSkeleton />
      ) : filtered.length === 0 ? (
        <EmptyState onCreate={() => setCreateOpen(true)} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
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
    </DashboardPageLayout>
  );
}

function VaultGridSkeleton() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="h-44 rounded-xl bg-pop animate-pulse" />
      ))}
    </div>
  );
}

function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center space-y-4">
      <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-pop border border-border/50">
        <HardDrive className="w-8 h-8 text-muted-foreground/50" />
      </div>
      <div>
        <p className="font-display text-xl uppercase">No vaults yet</p>
        <p className="text-sm text-muted-foreground mt-1">
          Create your first vault to start storing encrypted files
        </p>
      </div>
      <Button onClick={onCreate} variant="outline">
        <Plus />
        Create a vault
      </Button>
    </div>
  );
}
