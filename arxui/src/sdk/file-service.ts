import { createClient } from "@connectrpc/connect";
import { ArxService } from "@/src/gen/arx_connect";
import type { Transport } from "@connectrpc/connect";
import { mapStats, type VaultStats } from "./vault-service";

export interface FileEntry {
  path: string;
  size: bigint;
  mtime: bigint;
}

export interface ProgressItem {
  fileName: string;
  bytesUploaded: number;
  totalBytes: number;
  done: boolean;
}

export interface DiffEntry {
  kind: string;
  path: string;
}

export interface UploadFile {
  name: string; // path in archive
  file: File; // browser File object
}

export class FileService {
  private client: ReturnType<typeof createClient<typeof ArxService>>;

  constructor(
    transport: Transport,
    private baseUrl: string,
    private getToken: () => string | null,
  ) {
    this.client = createClient(ArxService, transport);
  }

  async list(
    vaultId: string,
    opts?: { prefix?: string; offset?: number; limit?: number },
  ): Promise<{ entries: FileEntry[]; total: number }> {
    const res = await this.client.crudLs({
      archiveId: vaultId,
      prefix: opts?.prefix ?? "",
      longFormat: true,
      offset: opts?.offset ?? 0,
      limit: opts?.limit ?? 0,
    });
    return {
      entries: res.entries.map((e) => ({
        path: e.path,
        size: e.size,
        mtime: e.mtime,
      })),
      total: res.totalCount,
    };
  }

  /**
   * Upload multiple files into a vault via HTTP multipart.
   *
   * grpc-web over HTTP/1.1 does not support client streaming (Fetch API
   * can't stream request bodies), so uploads go through a dedicated
   * axum endpoint `POST /api/upload/{vaultId}` on the same port.
   */
  async upload(
    vaultId: string,
    files: UploadFile[],
    opts?: { onProgress?: (item: ProgressItem) => void },
  ): Promise<void> {
    for (const uf of files) {
      await this.uploadSingle(vaultId, uf, opts?.onProgress);
    }
  }

  uploadSingle(
    vaultId: string,
    uf: UploadFile,
    onProgress?: (item: ProgressItem) => void,
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      const form = new FormData();
      form.append(uf.name, uf.file, uf.name);

      const xhr = new XMLHttpRequest();
      xhr.open("POST", `${this.baseUrl}/api/upload/${encodeURIComponent(vaultId)}`);

      const token = this.getToken();
      if (token) xhr.setRequestHeader("Authorization", `Bearer ${token}`);

      xhr.upload.onprogress = (e) => {
        if (e.lengthComputable) {
          onProgress?.({
            fileName: uf.name,
            bytesUploaded: e.loaded,
            totalBytes: e.total,
            done: false,
          });
        }
      };

      xhr.onload = () => {
        if (xhr.status < 400) {
          onProgress?.({
            fileName: uf.name,
            bytesUploaded: uf.file.size,
            totalBytes: uf.file.size,
            done: true,
          });
          resolve();
        } else {
          reject(new Error(xhr.responseText || `Upload failed (${xhr.status})`));
        }
      };
      xhr.onerror = () => reject(new Error("Upload failed (network error)"));
      xhr.onabort = () => reject(new Error("Upload aborted"));

      xhr.send(form);
    });
  }

  /**
   * Download a single file from a vault, returning a Blob.
   * Streams server-side DownloadFrames and concatenates data frames.
   */
  async download(
    vaultId: string,
    path: string,
    opts?: { onProgress?: (bytes: number) => void },
  ): Promise<Blob> {
    const chunks: Uint8Array[] = [];
    let totalBytes = 0;

    for await (const frame of this.client.extractStream({
      archiveId: vaultId,
      path,
      start: 0n,
      len: 0n,
    })) {
      if (frame.payload.case === "data") {
        chunks.push(frame.payload.value);
        totalBytes += frame.payload.value.length;
        opts?.onProgress?.(totalBytes);
      } else if (frame.payload.case === "error") {
        throw new Error(frame.payload.value);
      }
    }

    return new Blob(chunks.map((c) => new Uint8Array(c)));
  }

  async delete(vaultId: string, path: string, recursive = false): Promise<void> {
    const res = await this.client.crudRm({ archiveId: vaultId, path, recursive });
    if (!res.ok) throw new Error(res.error || "Delete failed");
  }

  async rename(vaultId: string, from: string, to: string): Promise<void> {
    const res = await this.client.crudMv({ archiveId: vaultId, from, to });
    if (!res.ok) throw new Error(res.error || "Rename failed");
  }

  async sync(
    vaultId: string,
  ): Promise<{ ok: boolean; sizeBytes: bigint; stats?: VaultStats }> {
    const res = await this.client.crudSync({ archiveId: vaultId, sealBase: false });
    if (res.error) throw new Error(res.error);
    return {
      ok: res.ok,
      sizeBytes: res.sizeBytes,
      stats: res.stats ? mapStats(res.stats) : undefined,
    };
  }

  /**
   * Stream the first `maxBytes` of a file for preview — used for text previews
   * without downloading the whole file. Returns the bytes + a `truncated` flag.
   */
  async preview(
    vaultId: string,
    path: string,
    maxBytes: number,
  ): Promise<{ bytes: Uint8Array; truncated: boolean; totalSize: number }> {
    const chunks: Uint8Array[] = [];
    let total = 0;
    let truncated = false;
    for await (const frame of this.client.extractStream({
      archiveId: vaultId,
      path,
      start: 0n,
      len: BigInt(maxBytes),
    })) {
      if (frame.payload.case === "data") {
        const bytes = frame.payload.value;
        chunks.push(bytes);
        total += bytes.length;
        if (total >= maxBytes) {
          truncated = true;
          break;
        }
      } else if (frame.payload.case === "error") {
        throw new Error(frame.payload.value);
      }
    }
    const out = new Uint8Array(total);
    let o = 0;
    for (const c of chunks) {
      out.set(c, o);
      o += c.length;
    }
    return { bytes: out, truncated, totalSize: total };
  }

  async diff(vaultId: string): Promise<DiffEntry[]> {
    const res = await this.client.crudDiff({ archiveId: vaultId });
    return res.entries.map((e) => ({ kind: e.kind, path: e.path }));
  }
}
