"use client";

import { useState } from "react";
import { useSdk } from "@/src/lib/sdk-context";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Loader2 } from "lucide-react";

interface Props {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onCreated: () => void;
}

export function CreateVaultDialog({ open, onOpenChange, onCreated }: Props) {
  const sdk = useSdk();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await sdk.vaults.create({
        name: name.trim(),
        password: password || undefined,
      });
      onCreated();
      onOpenChange(false);
      setName("");
      setPassword("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create vault");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Create a new vault</DialogTitle>
          <DialogDescription>
            A vault is an encrypted archive that stores your files. Files are
            compressed and deduplicated automatically.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleCreate} className="space-y-4 pt-2">
          <div className="space-y-2">
            <Label htmlFor="vault-name">Vault name</Label>
            <Input
              id="vault-name"
              placeholder="e.g. Holiday Photos"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              autoFocus
              disabled={loading}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="vault-password">
              Encryption password{" "}
              <span className="text-muted-foreground font-normal">(optional)</span>
            </Label>
            <Input
              id="vault-password"
              type="password"
              placeholder="Leave blank for unencrypted"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={loading}
            />
            <p className="text-xs text-muted-foreground">
              If set, you&apos;ll need this password to access files. Cannot be
              changed later.
            </p>
          </div>

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={loading || !name.trim()}>
              {loading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Creating…
                </>
              ) : (
                "Create vault"
              )}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
