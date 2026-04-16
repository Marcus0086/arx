"use client";

import { useState, use } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { useUploadStore } from "@/src/stores/upload-store";
import { FileGrid } from "@/components/arx/file-grid";
import { UploadZone } from "@/components/arx/upload-zone";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  ArrowLeft,
  RefreshCw,
  GitCompare,
  Upload,
  Loader2,
} from "lucide-react";
import { useRouter } from "next/navigation";
import type { UploadFile } from "@/src/sdk";

interface Props {
  params: Promise<{ vaultId: string }>;
}

export default function VaultDetailPage({ params }: Props) {
  const { vaultId } = use(params);
  const sdk = useSdk();
  const qc = useQueryClient();
  const { add: addToQueue, update: updateItem } = useUploadStore();
  const router = useRouter();
  const [uploading, setUploading] = useState(false);

  const { data: files = [], isLoading, refetch } = useQuery({
    queryKey: ["vault-files", vaultId],
    queryFn: () => sdk.files.list(vaultId),
  });

  const { data: diffs = [] } = useQuery({
    queryKey: ["vault-diff", vaultId],
    queryFn: () => sdk.files.diff(vaultId),
    staleTime: 10_000,
  });

  const syncMutation = useMutation({
    mutationFn: () => sdk.files.sync(vaultId),
    onSuccess: ({ newVaultId }) => {
      qc.invalidateQueries({ queryKey: ["vaults"] });
      router.replace(`/vaults/${newVaultId}`);
    },
  });

  const deleteFile = useMutation({
    mutationFn: (path: string) => sdk.files.delete(vaultId, path),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["vault-files", vaultId] }),
  });

  async function handleUpload(browserFiles: File[]) {
    setUploading(true);
    const items = addToQueue(browserFiles, vaultId);
    const uploadFiles: UploadFile[] = browserFiles.map((f, i) => ({
      name: f.name,
      file: f,
      _id: items[i].id,
    } as UploadFile & { _id: string }));

    for (let i = 0; i < uploadFiles.length; i++) {
      const uf = uploadFiles[i];
      const item = items[i];
      updateItem(item.id, { status: "uploading" });
      try {
        await sdk.files.upload(vaultId, [uf], {
          onProgress: ({ bytesUploaded }) =>
            updateItem(item.id, { bytesUploaded }),
        });
        updateItem(item.id, { status: "done", bytesUploaded: item.fileSize });
      } catch (err) {
        updateItem(item.id, {
          status: "error",
          error: err instanceof Error ? err.message : "Upload failed",
        });
      }
    }

    setUploading(false);
    qc.invalidateQueries({ queryKey: ["vault-files", vaultId] });
    qc.invalidateQueries({ queryKey: ["vaults"] });
  }

  async function handleDownload(path: string) {
    const blob = await sdk.files.download(vaultId, path);
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = path.split("/").pop() ?? path;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <UploadZone onDrop={handleUpload} disabled={uploading}>
      <div className="p-6 space-y-5 max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => router.push("/vaults")}
          >
            <ArrowLeft className="w-4 h-4" />
          </Button>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-bold tracking-tight font-mono text-xs text-muted-foreground">
                {vaultId.slice(0, 8)}…
              </h1>
              {diffs.length > 0 && (
                <Badge variant="outline" className="text-xs gap-1">
                  <GitCompare className="w-3 h-3" />
                  {diffs.length} pending change{diffs.length !== 1 ? "s" : ""}
                </Badge>
              )}
            </div>
            <p className="text-sm text-muted-foreground mt-0.5">
              {files.length} file{files.length !== 1 ? "s" : ""}
            </p>
          </div>

          <div className="flex items-center gap-2">
            {/* Upload button */}
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              disabled={uploading}
              onClick={() => {
                const inp = document.createElement("input");
                inp.type = "file";
                inp.multiple = true;
                inp.onchange = (e) => {
                  const fs = (e.target as HTMLInputElement).files;
                  if (fs && fs.length > 0) handleUpload(Array.from(fs));
                };
                inp.click();
              }}
            >
              {uploading ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Upload className="w-3.5 h-3.5" />
              )}
              Upload
            </Button>

            {/* Sync */}
            {diffs.length > 0 && (
              <Button
                size="sm"
                className="gap-1.5"
                onClick={() => syncMutation.mutate()}
                disabled={syncMutation.isPending}
              >
                {syncMutation.isPending ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <RefreshCw className="w-3.5 h-3.5" />
                )}
                Sync vault
              </Button>
            )}

            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => refetch()}
            >
              <RefreshCw className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {/* Files */}
        {isLoading ? (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {Array.from({ length: 12 }).map((_, i) => (
              <div key={i} className="aspect-square rounded-lg bg-muted/40 animate-pulse" />
            ))}
          </div>
        ) : (
          <FileGrid
            vaultId={vaultId}
            files={files}
            onDownload={handleDownload}
            onDelete={(path) => deleteFile.mutate(path)}
          />
        )}

        {files.length === 0 && !isLoading && (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <p className="text-muted-foreground text-sm">
              Drop files here or click Upload to add files to this vault
            </p>
          </div>
        )}
      </div>
    </UploadZone>
  );
}
