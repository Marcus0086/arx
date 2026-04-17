"use client";

import { useState, use, useRef, useEffect } from "react";
import {
  useInfiniteQuery,
  useQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { useUploadStore } from "@/src/stores/upload-store";
import { FileGrid } from "@/components/arx/file-grid";
import { UploadZone } from "@/components/arx/upload-zone";
import { DashboardPageLayout } from "@/components/arx/dashboard-page-layout";
import { FilePreviewSheet } from "@/components/arx/file-preview-sheet";
import { VaultStatsStrip } from "@/components/arx/vault-stats-strip";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Archive,
  ArrowLeft,
  Check,
  GitCompare,
  Loader2,
  Pencil,
  RefreshCw,
  Upload,
  X,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import type { FileEntry, UploadFile, Vault } from "@/src/sdk";
import type { UploadItem } from "@/src/stores/upload-store";

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
  const [preview, setPreview] = useState<FileEntry | null>(null);

  // Always keep the vault list fresh so stats + names update across pages.
  const { data: vaults = [] } = useQuery({
    queryKey: ["vaults"],
    queryFn: () => sdk.vaults.list(),
  });
  const vault: Vault | undefined = vaults.find((v) => v.id === vaultId);
  const vaultName = vault?.name?.toUpperCase() ?? vaultId.slice(0, 8) + "…";

  const PAGE_SIZE = 100;
  const sentinelRef = useRef<HTMLDivElement>(null);

  const {
    data: filePages,
    isLoading,
    isFetchingNextPage,
    fetchNextPage,
    hasNextPage,
    refetch,
  } = useInfiniteQuery({
    queryKey: ["vault-files-paged", vaultId],
    initialPageParam: 0,
    queryFn: ({ pageParam }) =>
      sdk.files.list(vaultId, { offset: pageParam, limit: PAGE_SIZE }),
    getNextPageParam: (lastPage, allPages) => {
      if (!lastPage) return undefined;
      const loaded = allPages.reduce((n, p) => n + (p?.entries?.length ?? 0), 0);
      return loaded < lastPage.total ? loaded : undefined;
    },
  });

  const files: FileEntry[] = filePages?.pages.flatMap((p) => p.entries) ?? [];

  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting && hasNextPage && !isFetchingNextPage) fetchNextPage();
      },
      { threshold: 0.1 },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  const { data: diffs = [] } = useQuery({
    queryKey: ["vault-diff", vaultId],
    queryFn: () => sdk.files.diff(vaultId),
    staleTime: 10_000,
  });

  const syncMutation = useMutation({
    mutationFn: () => sdk.files.sync(vaultId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["vaults"] });
      qc.invalidateQueries({ queryKey: ["vault-files-paged", vaultId] });
      qc.invalidateQueries({ queryKey: ["vault-diff", vaultId] });
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Sync failed"),
  });

  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const deleteFile = useMutation({
    mutationFn: (path: string) => sdk.files.delete(vaultId, path),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["vault-files-paged", vaultId] }),
    onError: (e) => toast.error(e instanceof Error ? e.message : "Delete failed"),
  });

  function requestDelete(path: string) {
    setConfirmDelete(path);
  }

  function confirmDeleteFile() {
    if (confirmDelete) {
      deleteFile.mutate(confirmDelete);
      setConfirmDelete(null);
      setPreview(null);
    }
  }

  const renameMutation = useMutation({
    mutationFn: (name: string) => sdk.vaults.rename(vaultId, name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["vaults"] }),
    onError: (e) => toast.error(e instanceof Error ? e.message : "Rename failed"),
  });

  const UPLOAD_CONCURRENCY = 20;

  async function handleUpload(browserFiles: File[]) {
    setUploading(true);
    const items = addToQueue(browserFiles, vaultId);

    const tasks: Array<{ uf: UploadFile; item: UploadItem }> = browserFiles.map(
      (f, i) => ({
        uf: { name: f.name, file: f },
        item: items[i],
      }),
    );
    const queue = tasks.slice();

    async function worker() {
      let task: (typeof tasks)[number] | undefined;
      while ((task = queue.shift()) !== undefined) {
        const { uf, item } = task;
        updateItem(item.id, { status: "uploading" });
        try {
          await sdk.files.uploadSingle(vaultId, uf, ({ bytesUploaded }) =>
            updateItem(item.id, { bytesUploaded }),
          );
          updateItem(item.id, { status: "done", bytesUploaded: item.fileSize });
        } catch (err) {
          updateItem(item.id, {
            status: "error",
            error: err instanceof Error ? err.message : "Upload failed",
          });
        }
      }
    }

    await Promise.all(
      Array.from({ length: Math.min(UPLOAD_CONCURRENCY, tasks.length) }, worker),
    );

    setUploading(false);
    qc.invalidateQueries({ queryKey: ["vault-files-paged", vaultId] });
    qc.invalidateQueries({ queryKey: ["vault-diff", vaultId] });
    qc.invalidateQueries({ queryKey: ["vaults"] });
  }

  async function handleDownload(path: string) {
    try {
      const blob = await sdk.files.download(vaultId, path);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = path.split("/").pop() ?? path;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Download failed");
    }
  }

  const fileCount = filePages?.pages[0]?.total ?? files.length;
  const description = `${fileCount} file${fileCount !== 1 ? "s" : ""}${
    diffs.length > 0 ? ` · ${diffs.length} PENDING` : ""
  }`;

  return (
    <UploadZone onDrop={handleUpload} disabled={uploading}>
      <DashboardPageLayout header={{ title: vaultName, icon: Archive, description }}>
        {/* Title row with inline rename */}
        <div className="flex items-center gap-2 flex-wrap">
          <Button variant="ghost" size="sm" onClick={() => router.push("/vaults")}>
            <ArrowLeft />
            Back
          </Button>

          <InlineRename
            value={vault?.name ?? ""}
            onSave={(name) => renameMutation.mutate(name)}
            disabled={!vault}
          />

          {diffs.length > 0 && (
            <Badge variant="outline-warning">
              <GitCompare />
              {diffs.length} PENDING
            </Badge>
          )}

          <div className="ml-auto flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
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
              {uploading ? <Loader2 className="animate-spin" /> : <Upload />}
              Upload
            </Button>

            {diffs.length > 0 && (
              <Button
                size="sm"
                onClick={() => syncMutation.mutate()}
                disabled={syncMutation.isPending}
              >
                {syncMutation.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <RefreshCw />
                )}
                Sync
              </Button>
            )}

            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => refetch()}
              aria-label="Refresh"
            >
              <RefreshCw />
            </Button>
          </div>
        </div>

        {/* Stats strip */}
        {vault?.stats && <VaultStatsStrip stats={vault.stats} />}

        {/* Files */}
        {isLoading ? (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
            {Array.from({ length: 10 }).map((_, i) => (
              <div key={i} className="aspect-square rounded-lg bg-pop animate-pulse" />
            ))}
          </div>
        ) : files.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center gap-3">
            <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-pop border border-border/50">
              <Upload className="w-8 h-8 text-muted-foreground/50" />
            </div>
            <p className="font-display text-xl uppercase">Drop files to upload</p>
            <p className="text-sm text-muted-foreground">
              Drag files anywhere, or click Upload above
            </p>
          </div>
        ) : (
          <>
            <FileGrid
              vaultId={vaultId}
              files={files}
              onPreview={(f) => setPreview(f)}
              onDownload={handleDownload}
              onDelete={requestDelete}
            />
            <div ref={sentinelRef} className="h-4" />
            {isFetchingNextPage && (
              <div className="flex justify-center py-4">
                <Loader2 className="animate-spin" />
              </div>
            )}
            {!hasNextPage && files.length > 0 && !isFetchingNextPage && (
              <p className="text-xs text-muted-foreground text-center py-4 uppercase tracking-wide">
                All {fileCount} files loaded
              </p>
            )}
          </>
        )}
      </DashboardPageLayout>

      <FilePreviewSheet
        vaultId={vaultId}
        file={preview}
        onClose={() => setPreview(null)}
        onDownload={handleDownload}
        onDelete={requestDelete}
        deleteIsPending={deleteFile.isPending}
      />

      {/* Confirm delete dialog */}
      {confirmDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div className="bg-card border border-border rounded-lg p-6 max-w-sm w-full mx-4 flex flex-col gap-4">
            <p className="font-display text-lg uppercase">Delete file?</p>
            <p className="text-sm text-muted-foreground break-all">
              <span className="font-mono text-foreground">
                {confirmDelete.split("/").pop()}
              </span>{" "}
              will be permanently deleted.
            </p>
            <div className="flex gap-2 justify-end">
              <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(null)}>
                Cancel
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive hover:text-destructive"
                onClick={confirmDeleteFile}
                disabled={deleteFile.isPending}
              >
                {deleteFile.isPending ? <Loader2 className="animate-spin" /> : null}
                Delete
              </Button>
            </div>
          </div>
        </div>
      )}
    </UploadZone>
  );
}

function InlineRename({
  value,
  onSave,
  disabled,
}: {
  value: string;
  onSave: (name: string) => void;
  disabled?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!editing) setDraft(value);
  }, [value, editing]);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  function commit() {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== value) onSave(trimmed);
    setEditing(false);
  }

  if (disabled) return null;
  if (!editing) {
    return (
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          setDraft(value);
          setEditing(true);
        }}
        className="text-muted-foreground hover:text-foreground"
      >
        <Pencil />
        Rename
      </Button>
    );
  }
  return (
    <div className="flex items-center gap-1">
      <input
        ref={inputRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") setEditing(false);
        }}
        maxLength={128}
        className="h-8 px-2 rounded bg-card border border-border text-sm font-display uppercase outline-none focus:border-primary"
      />
      <Button variant="ghost" size="icon-sm" onClick={commit}>
        <Check />
      </Button>
      <Button variant="ghost" size="icon-sm" onClick={() => setEditing(false)}>
        <X />
      </Button>
    </div>
  );
}
