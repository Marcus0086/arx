import { createClient } from "@connectrpc/connect";
import type { PartialMessage } from "@bufbuild/protobuf";
import { ArxService } from "@/src/gen/arx_connect";
import type { UploadFrame } from "@/src/gen/arx_pb";
import type { Transport } from "@connectrpc/connect";

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

const CHUNK_SIZE = 512 * 1024; // 512 KB — matches server frame limit

export class FileService {
  private client: ReturnType<typeof createClient<typeof ArxService>>;

  constructor(transport: Transport) {
    this.client = createClient(ArxService, transport);
  }

  async list(vaultId: string, prefix = ""): Promise<FileEntry[]> {
    const res = await this.client.crudLs({
      archiveId: vaultId,
      prefix,
      longFormat: true,
    });
    return res.entries.map((e) => ({
      path: e.path,
      size: e.size,
      mtime: e.mtime,
    }));
  }

  /**
   * Upload multiple files into a vault using the CrudAddStream RPC.
   * Streams each file in 512 KB chunks and fires onProgress callbacks.
   */
  async upload(
    vaultId: string,
    files: UploadFile[],
    opts?: { onProgress?: (item: ProgressItem) => void },
  ): Promise<void> {
    // connect-web client-streaming: collect all frames then call once
    // Since connect-web doesn't support true client-streaming in browsers
    // (HTTP/1.1 doesn't support bidirectional streaming), we use PackStream
    // which accepts a sequence of UploadFrames.
    //
    // We buffer each file's content and send as a single call per vault.
    for (const uf of files) {
      await this._uploadOne(vaultId, uf, opts?.onProgress);
    }
  }

  private async _uploadOne(
    vaultId: string,
    uf: UploadFile,
    onProgress?: (item: ProgressItem) => void,
  ): Promise<void> {
    const buffer = await uf.file.arrayBuffer();
    const bytes = new Uint8Array(buffer);

    const self = this;
    // Build an async generator of PartialMessage<UploadFrame> frames
    async function* frameStream(): AsyncIterable<PartialMessage<UploadFrame>> {
      // Frame 1: CrudHeader
      yield { payload: { case: "crudHeader", value: { archiveId: vaultId } } };

      // Frame 2: FileInfo
      yield {
        payload: {
          case: "fileInfo",
          value: {
            path: uf.name,
            mode: 0o644,
            mtime: BigInt(Math.floor(uf.file.lastModified / 1000)),
            size: BigInt(uf.file.size),
          },
        },
      };

      // Data frames (512 KB chunks)
      let offset = 0;
      while (offset < bytes.length) {
        const chunk = bytes.slice(offset, offset + CHUNK_SIZE);
        yield { payload: { case: "data", value: chunk } };
        offset += chunk.length;
        onProgress?.({
          fileName: uf.name,
          bytesUploaded: Math.min(offset, bytes.length),
          totalBytes: bytes.length,
          done: false,
        });
        // Yield control back to allow progress UI updates
        await new Promise<void>((r) => setTimeout(r, 0));
      }

      // Finalize
      yield { payload: { case: "finalize", value: true } };
    }

    void self; // suppress unused warning
    await this.client.crudAddStream(frameStream());

    onProgress?.({
      fileName: uf.name,
      bytesUploaded: bytes.length,
      totalBytes: bytes.length,
      done: true,
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

  async sync(vaultId: string): Promise<{ newVaultId: string }> {
    const res = await this.client.crudSync({ archiveId: vaultId, sealBase: false });
    if (res.error) throw new Error(res.error);
    return { newVaultId: res.newArchiveId };
  }

  async diff(vaultId: string): Promise<DiffEntry[]> {
    const res = await this.client.crudDiff({ archiveId: vaultId });
    return res.entries.map((e) => ({ kind: e.kind, path: e.path }));
  }
}
