"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSdk } from "@/src/lib/sdk-context";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Download, Film, FileText, Image, Music, Package, Trash2 } from "lucide-react";
import type { FileEntry } from "@/src/sdk";
import { formatDistanceToNow } from "date-fns";

interface FileGridProps {
  vaultId: string;
  files: FileEntry[];
  onDownload: (path: string) => void;
  onDelete: (path: string) => void;
}

export function FileGrid({ vaultId, files, onDownload, onDelete }: FileGridProps) {
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <>
      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
        {files.map((file) => (
          <FileItem
            key={file.path}
            vaultId={vaultId}
            file={file}
            selected={selected === file.path}
            onSelect={() => setSelected(file.path === selected ? null : file.path)}
            onDownload={() => onDownload(file.path)}
            onDelete={() => onDelete(file.path)}
          />
        ))}
      </div>
    </>
  );
}

interface FileItemProps {
  vaultId: string;
  file: FileEntry;
  selected: boolean;
  onSelect: () => void;
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

function FileItem({
  vaultId,
  file,
  selected,
  onSelect,
  onDownload,
  onDelete,
}: FileItemProps) {
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
          className={`group flex flex-col gap-1.5 p-2 rounded-xl border cursor-pointer transition-all ${
            selected
              ? "border-primary bg-primary/5"
              : "border-border/40 bg-card hover:border-border hover:bg-muted/30"
          }`}
          onClick={onSelect}
          onDoubleClick={onDownload}
        >
          {/* Thumbnail or icon */}
          <div className="aspect-square rounded-lg overflow-hidden flex items-center justify-center bg-muted/40">
            {isImage && previewUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={previewUrl} alt={name} className="w-full h-full object-cover" />
            ) : isVideo ? (
              <Film className="w-8 h-8 text-muted-foreground/50" />
            ) : AUDIO_EXTS.has(ext) ? (
              <Music className="w-8 h-8 text-muted-foreground/50" />
            ) : ARCHIVE_EXTS.has(ext) ? (
              <Package className="w-8 h-8 text-muted-foreground/50" />
            ) : ["txt", "md", "json", "yaml", "toml", "rs", "ts", "tsx", "js"].includes(
                ext,
              ) ? (
              <FileText className="w-8 h-8 text-muted-foreground/50" />
            ) : (
              <Image className="w-8 h-8 text-muted-foreground/50" />
            )}
          </div>

          {/* Name & size */}
          <div className="space-y-0.5 min-w-0">
            <p className="text-xs font-medium truncate leading-tight">{name}</p>
            <p className="text-[10px] text-muted-foreground">{formatBytes(file.size)}</p>
          </div>
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent className="w-40">
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
