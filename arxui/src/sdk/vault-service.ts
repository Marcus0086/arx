import { createClient } from "@connectrpc/connect";
import { ArxService } from "@/src/gen/arx_connect";
import type { Transport } from "@connectrpc/connect";

export interface Vault {
  id: string;
  name: string;
  sizeBytes: bigint;
  createdAt: string;
  encrypted: boolean;
}

export interface CreateVaultOpts {
  name: string;
  label?: string;
  owner?: string;
  notes?: string;
  password?: string;
  deterministic?: boolean;
}

export interface VerifyResult {
  ok: boolean;
  error: string;
}

export class VaultService {
  private client: ReturnType<typeof createClient<typeof ArxService>>;

  constructor(transport: Transport) {
    this.client = createClient(ArxService, transport);
  }

  async list(): Promise<Vault[]> {
    const res = await this.client.listArchives({});
    return res.archives.map((a) => ({
      id: a.id,
      name: a.name,
      sizeBytes: a.sizeBytes,
      createdAt: a.createdAt,
      encrypted: a.encrypted,
    }));
  }

  async create(opts: CreateVaultOpts): Promise<Vault> {
    const res = await this.client.issue({
      archiveName: opts.name,
      label: opts.label ?? "",
      owner: opts.owner ?? "",
      notes: opts.notes ?? "",
      deterministic: opts.deterministic ?? false,
      key: opts.password
        ? { keySource: { case: "password", value: opts.password } }
        : undefined,
    });
    if (res.error) throw new Error(res.error);
    return { id: res.archiveId, name: opts.name, sizeBytes: 0n, createdAt: new Date().toISOString(), encrypted: !!opts.password };
  }

  async delete(vaultId: string): Promise<void> {
    const res = await this.client.deleteArchive({ archiveId: vaultId });
    if (!res.ok) throw new Error(res.error || "Delete failed");
  }

  async verify(vaultId: string): Promise<VerifyResult> {
    const res = await this.client.verify({ archiveId: vaultId });
    return { ok: res.ok, error: res.error };
  }
}
