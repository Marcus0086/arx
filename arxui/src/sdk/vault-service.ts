import { createClient } from "@connectrpc/connect";
import { ArxService } from "@/src/gen/arx_connect";
import type { ArchiveStats as ProtoArchiveStats } from "@/src/gen/arx_pb";
import type { Transport } from "@connectrpc/connect";

export interface VaultStats {
  files: number;
  dirs: number;
  chunks: number;
  logicalBytes: bigint;
  storedBytes: bigint;
  compressionRatio: number;
  savingsBytes: bigint;
}

export interface Vault {
  id: string;
  name: string;
  sizeBytes: bigint;
  createdAt: string;
  encrypted: boolean;
  stats?: VaultStats;
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

/** Convert a protobuf int64 epoch-seconds field (bigint | string | number) to ISO string. */
function protoTimestampToIso(v: bigint | string | number): string {
  const secs = typeof v === "bigint" ? Number(v) : Number(v);
  if (!isFinite(secs) || secs <= 0) return "";
  return new Date(secs * 1000).toISOString();
}

/** Map the generated proto stats to a camelCase VaultStats object. */
export function mapStats(s: ProtoArchiveStats): VaultStats {
  return {
    files: Number(s.files),
    dirs: Number(s.dirs),
    chunks: Number(s.chunks),
    logicalBytes: s.logicalBytes,
    storedBytes: s.storedBytes,
    compressionRatio: s.compressionRatio,
    savingsBytes: s.savingsBytes,
  };
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
      createdAt: protoTimestampToIso(a.createdAt),
      encrypted: a.encrypted,
      stats: a.stats ? mapStats(a.stats) : undefined,
    }));
  }

  async create(opts: CreateVaultOpts): Promise<Vault> {
    const res = await this.client.issue({
      archiveName: opts.name,
      label: opts.label ?? opts.name,
      owner: opts.owner ?? "",
      notes: opts.notes ?? "",
      deterministic: opts.deterministic ?? false,
      key: opts.password
        ? { keySource: { case: "password", value: opts.password } }
        : undefined,
    });
    if (res.error) throw new Error(res.error);
    return {
      id: res.archiveId,
      name: opts.name,
      sizeBytes: 0n,
      createdAt: new Date().toISOString(),
      encrypted: !!opts.password,
    };
  }

  async delete(vaultId: string): Promise<void> {
    const res = await this.client.deleteArchive({ archiveId: vaultId });
    if (!res.ok) throw new Error(res.error || "Delete failed");
  }

  async rename(vaultId: string, name: string): Promise<void> {
    const res = await this.client.renameArchive({ archiveId: vaultId, name });
    if (!res.ok) throw new Error(res.error || "Rename failed");
  }

  async verify(vaultId: string): Promise<VerifyResult> {
    const res = await this.client.verify({ archiveId: vaultId });
    return { ok: res.ok, error: res.error };
  }
}
