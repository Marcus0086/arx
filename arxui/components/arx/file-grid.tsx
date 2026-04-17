"use client";

import { useRef, useEffect, useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useSdk } from "@/src/lib/sdk-context";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Download,
  Eye,
  Film,
  FileText,
  Image,
  Music,
  Package,
  Trash2,
} from "lucide-react";
import type { FileEntry } from "@/src/sdk";

interface FileGridProps {
  vaultId: string;
  files: FileEntry[];
  onPreview: (file: FileEntry) => void;
  onDownload: (path: string) => void;
  onDelete: (path: string) => void;
}

function getColCount(width: number): number {
  if (width < 640) return 2;
  if (width < 768) return 3;
  if (width < 1024) return 4;
  return 5;
}

export function FileGrid({
  vaultId,
  files,
  onPreview,
  onDownload,
  onDelete,
}: FileGridProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) =>
      setContainerWidth(entry.contentRect.width),
    );
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const colCount = containerWidth > 0 ? getColCount(containerWidth) : 5;
  const rowHeight = containerWidth > 0 ? Math.floor(containerWidth / colCount) + 44 : 204;

  // Chunk files into rows
  const rows: FileEntry[][] = [];
  for (let i = 0; i < files.length; i += colCount) {
    rows.push(files.slice(i, i + colCount));
  }

  const estimateSize = useCallback(() => rowHeight, [rowHeight]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => containerRef.current,
    estimateSize,
    overscan: 3,
  });

  return (
    <div
      ref={containerRef}
      style={{
        height: "calc(100vh - 320px)",
        minHeight: 300,
        overflow: "auto",
        position: "relative",
      }}
    >
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((vRow) => (
          <div
            key={vRow.index}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: vRow.size,
              transform: `translateY(${vRow.start}px)`,
              display: "grid",
              gridTemplateColumns: `repeat(${colCount}, 1fr)`,
              gap: "1rem",
              alignItems: "start",
            }}
          >
            {rows[vRow.index].map((file) => (
              <FileItem
                key={file.path}
                vaultId={vaultId}
                file={file}
                onPreview={() => onPreview(file)}
                onDownload={() => onDownload(file.path)}
                onDelete={() => onDelete(file.path)}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

interface FileItemProps {
  vaultId: string;
  file: FileEntry;
  onPreview: () => void;
  onDownload: () => void;
  onDelete: () => void;
}

const IMAGE_EXTS = new Set(["jpg", "jpeg", "png", "gif", "webp", "avif", "svg", "bmp"]);
const VIDEO_EXTS = new Set(["mp4", "mov", "avi", "mkv", "webm", "m4v"]);
const AUDIO_EXTS = new Set(["mp3", "wav", "flac", "m4a", "ogg", "aac"]);
const ARCHIVE_EXTS = new Set(["zip", "tar", "gz", "arx", "7z", "rar"]);

function getExt(path: string) {
  return path.split(".").pop()?.toLowerCase() ?? "";
}

function FileItem({ vaultId, file, onPreview, onDownload, onDelete }: FileItemProps) {
  const ext = getExt(file.path);
  const isImage = IMAGE_EXTS.has(ext);
  const isVideo = VIDEO_EXTS.has(ext);

  const sdk = useSdk();

  // Lazy-load thumbnail for images
  const { data: previewUrl } = useQuery({
    queryKey: ["preview", vaultId, file.path],
    queryFn: async () => {
      const blob = await sdk.files.download(vaultId, file.path);
      return URL.createObjectURL(blob);
    },
    enabled: isImage && Number(file.size) < 8 * 1024 * 1024, // only for files < 8 MB
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });

  const name = file.path.split("/").pop() ?? file.path;

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          className="group relative flex flex-col gap-1 p-1.5 rounded-lg cursor-pointer transition-all bg-pop ring-1 ring-transparent hover:ring-border"
          onClick={onPreview}
        >
          {/* Hover delete button */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            className="absolute top-2 right-2 z-10 opacity-0 group-hover:opacity-100 transition-opacity
                       p-1 rounded bg-background/80 text-destructive hover:bg-destructive hover:text-white"
            aria-label="Delete"
          >
            <Trash2 className="size-3" />
          </button>

          {/* Thumbnail or icon */}
          <div className="aspect-square rounded overflow-hidden flex items-center justify-center bg-card">
            {isImage && previewUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={previewUrl} alt={name} className="w-full h-full object-cover" />
            ) : isVideo ? (
              <Film className="size-8 text-muted-foreground/50" />
            ) : AUDIO_EXTS.has(ext) ? (
              <Music className="size-8 text-muted-foreground/50" />
            ) : ARCHIVE_EXTS.has(ext) ? (
              <Package className="size-8 text-muted-foreground/50" />
            ) : ext === "pdf" ? (
              <FileText className="size-8 text-red-500/70" />
            ) : ["txt", "md", "json", "yaml", "toml", "rs", "ts", "tsx", "js"].includes(
                ext,
              ) ? (
              <FileText className="size-8 text-muted-foreground/50" />
            ) : (
              <Image className="size-8 text-muted-foreground/50" />
            )}
          </div>

          {/* Name & size */}
          <div className="flex flex-col gap-0.5 min-w-0 px-1 pb-0.5">
            <p className="text-xs truncate leading-tight">{name}</p>
            <p className="text-[10px] uppercase text-muted-foreground tracking-wide">
              {formatBytes(file.size)}
            </p>
          </div>
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent className="w-44">
        <ContextMenuItem onClick={onPreview} className="gap-2 text-sm">
          <Eye className="w-3.5 h-3.5" />
          Preview
        </ContextMenuItem>
        <ContextMenuItem onClick={onDownload} className="gap-2 text-sm">
          <Download className="w-3.5 h-3.5" />
          Download
        </ContextMenuItem>
        <ContextMenuItem
          onClick={onDelete}
          className="gap-2 text-sm text-destructive focus:text-destructive"
        >
          <Trash2 className="w-3.5 h-3.5" />
          Delete
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

function formatBytes(n: bigint | number) {
  const bytes = typeof n === "bigint" ? Number(n) : n;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}
