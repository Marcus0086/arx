"use client";

import { useEffect, useState } from "react";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Bullet } from "@/components/ui/bullet";
import { useSdk } from "@/src/lib/sdk-context";
import type { FileEntry } from "@/src/sdk";
import {
  Download,
  File,
  FileText,
  Film,
  Image as ImageIcon,
  Loader2,
  Music,
  Package,
  Trash2,
} from "lucide-react";

const IMAGE_EXTS = new Set(["jpg", "jpeg", "png", "gif", "webp", "avif", "svg", "bmp"]);
const VIDEO_EXTS = new Set(["mp4", "mov", "webm", "mkv", "m4v"]);
const AUDIO_EXTS = new Set(["mp3", "wav", "flac", "m4a", "ogg", "aac"]);
const PDF_EXTS = new Set(["pdf"]);
const TEXT_EXTS = new Set([
  "txt",
  "md",
  "json",
  "yaml",
  "yml",
  "toml",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "py",
  "sh",
  "log",
  "csv",
  "xml",
  "html",
  "css",
  "go",
  "java",
]);
const ARCHIVE_EXTS = new Set(["zip", "tar", "gz", "arx", "7z", "rar"]);

const TEXT_PREVIEW_BYTES = 64 * 1024;

function getExt(path: string): string {
  return path.split(".").pop()?.toLowerCase() ?? "";
}

function formatBytes(n: bigint | number) {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

function fmtDate(mtime: bigint): string {
  const secs = Number(mtime);
  if (!isFinite(secs) || secs <= 0) return "";
  return new Date(secs * 1000).toLocaleString();
}

interface Props {
  vaultId: string;
  file: FileEntry | null;
  onClose: () => void;
  onDownload: (path: string) => void;
  onDelete: (path: string) => void;
  deleteIsPending?: boolean;
}

export function FilePreviewSheet({
  vaultId,
  file,
  onClose,
  onDownload,
  onDelete,
  deleteIsPending,
}: Props) {
  return (
    <Sheet open={!!file} onOpenChange={(o) => !o && onClose()}>
      <SheetContent side="right" className="w-full sm:max-w-xl p-0 gap-0 flex flex-col">
        {file && (
          <>
            <SheetHeader className="px-4 py-3 border-b border-border/40 shrink-0 space-y-1">
              <SheetTitle className="font-display text-xl uppercase truncate">
                {file.path.split("/").pop() ?? file.path}
              </SheetTitle>
              <SheetDescription className="sr-only">
                Preview of {file.path} ({formatBytes(file.size)})
              </SheetDescription>
              <div className="text-xs uppercase text-muted-foreground flex items-center gap-2 flex-wrap">
                <Bullet size="sm" />
                <span>{formatBytes(file.size)}</span>
                <span>·</span>
                <span className="truncate">{file.path}</span>
              </div>
            </SheetHeader>

            <div className="flex-1 overflow-auto p-4">
              <FilePreview vaultId={vaultId} file={file} />
            </div>

            <div className="px-4 py-3 border-t border-border/40 shrink-0 flex items-center gap-2 bg-card">
              <MetadataTile label="Modified" value={fmtDate(file.mtime)} />
              <div className="ml-auto flex items-center gap-2">
                <Button variant="outline" size="sm" onClick={() => onDownload(file.path)}>
                  <Download />
                  Download
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-destructive hover:text-destructive"
                  disabled={deleteIsPending}
                  onClick={() => onDelete(file.path)}
                >
                  {deleteIsPending ? <Loader2 className="animate-spin" /> : <Trash2 />}
                  Delete
                </Button>
              </div>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

function MetadataTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col min-w-0">
      <span className="text-[10px] uppercase text-muted-foreground tracking-wide">
        {label}
      </span>
      <span className="text-xs font-mono truncate">{value || "—"}</span>
    </div>
  );
}

function FilePreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const ext = getExt(file.path);
  if (IMAGE_EXTS.has(ext)) return <ImagePreview vaultId={vaultId} file={file} />;
  if (VIDEO_EXTS.has(ext)) return <VideoPreview vaultId={vaultId} file={file} />;
  if (AUDIO_EXTS.has(ext)) return <AudioPreview vaultId={vaultId} file={file} />;
  if (PDF_EXTS.has(ext)) return <PdfPreview vaultId={vaultId} file={file} />;
  if (TEXT_EXTS.has(ext)) return <TextPreview vaultId={vaultId} file={file} />;
  return <UnknownPreview file={file} />;
}

function useBlobUrl(vaultId: string, path: string, enabled: boolean) {
  const sdk = useSdk();
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    let objectUrl: string | null = null;
    setLoading(true);
    setError(null);
    sdk.files
      .download(vaultId, path)
      .then((blob) => {
        if (cancelled) return;
        objectUrl = URL.createObjectURL(blob);
        setUrl(objectUrl);
        setLoading(false);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : "Failed to load");
        setLoading(false);
      });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [sdk, vaultId, path, enabled]);

  return { url, loading, error };
}

function ImagePreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const { url, loading, error } = useBlobUrl(vaultId, file.path, true);
  if (loading) return <Loading icon={<ImageIcon className="size-10 opacity-50" />} />;
  if (error) return <ErrorView msg={error} />;
  return (
    <div className="flex items-center justify-center bg-black/20 rounded overflow-hidden">
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={url!}
        alt={file.path}
        className="max-w-full max-h-[70vh] object-contain"
      />
    </div>
  );
}

function VideoPreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const { url, loading, error } = useBlobUrl(vaultId, file.path, true);
  if (loading) return <Loading icon={<Film className="size-10 opacity-50" />} />;
  if (error) return <ErrorView msg={error} />;
  return (
    <video controls src={url!} className="w-full max-h-[70vh] rounded bg-black/20" />
  );
}

function PdfPreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const { url, loading, error } = useBlobUrl(vaultId, file.path, true);
  if (loading) return <Loading icon={<FileText className="size-10 opacity-50" />} />;
  if (error) return <ErrorView msg={error} />;
  // Native browser PDF viewer via iframe + Blob URL. `#toolbar=1` is a hint
  // to Chromium/Firefox PDF viewers to show their built-in controls.
  return (
    <iframe
      title={file.path}
      src={`${url!}#toolbar=1&view=FitH`}
      className="w-full h-[75vh] rounded bg-card border border-border/30"
    />
  );
}

function AudioPreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const { url, loading, error } = useBlobUrl(vaultId, file.path, true);
  if (loading) return <Loading icon={<Music className="size-10 opacity-50" />} />;
  if (error) return <ErrorView msg={error} />;
  return (
    <div className="flex items-center justify-center py-10 bg-card rounded">
      <div className="flex flex-col items-center gap-4 w-full max-w-md px-6">
        <Music className="size-16 opacity-40" />
        <audio controls src={url!} className="w-full" />
      </div>
    </div>
  );
}

function TextPreview({ vaultId, file }: { vaultId: string; file: FileEntry }) {
  const sdk = useSdk();
  const [text, setText] = useState<string | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    sdk.files
      .preview(vaultId, file.path, TEXT_PREVIEW_BYTES)
      .then(({ bytes, truncated }) => {
        if (cancelled) return;
        setText(new TextDecoder("utf-8", { fatal: false }).decode(bytes));
        setTruncated(truncated);
        setLoading(false);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : "Failed to load");
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [sdk, vaultId, file.path]);

  if (loading) return <Loading icon={<FileText className="size-10 opacity-50" />} />;
  if (error) return <ErrorView msg={error} />;

  return (
    <div className="flex flex-col gap-2 h-full">
      <pre className="font-mono text-xs leading-relaxed bg-card rounded p-3 overflow-auto whitespace-pre-wrap wrap-break-word max-h-[65vh]">
        {text}
      </pre>
      {truncated && (
        <Badge variant="outline-warning" className="self-start">
          Truncated · first {formatBytes(TEXT_PREVIEW_BYTES)} shown (
          {formatBytes(file.size)} total)
        </Badge>
      )}
    </div>
  );
}

function UnknownPreview({ file }: { file: FileEntry }) {
  const ext = getExt(file.path);
  const Icon = ARCHIVE_EXTS.has(ext) ? Package : File;
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-4 text-center">
      <Icon className="size-16 opacity-40" />
      <div>
        <p className="font-display text-xl uppercase">{ext || "Binary"} File</p>
        <p className="text-xs uppercase text-muted-foreground mt-1 tracking-wide">
          No inline preview available · {formatBytes(file.size)}
        </p>
      </div>
    </div>
  );
}

function Loading({ icon }: { icon: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-3 text-muted-foreground">
      {icon}
      <Loader2 className="size-4 animate-spin" />
    </div>
  );
}

function ErrorView({ msg }: { msg: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-2 text-center">
      <p className="text-xs uppercase text-destructive">Failed to load</p>
      <p className="text-xs text-muted-foreground">{msg}</p>
    </div>
  );
}
