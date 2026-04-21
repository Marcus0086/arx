"use client";

import { useQuery } from "@tanstack/react-query";
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
  selectedPaths?: Set<string>;
  onToggleSelect?: (path: string, shift: boolean) => void;
  onPreview: (file: FileEntry) => void;
  onDownload: (path: string) => void;
  onDelete: (path: string) => void;
}

export function FileGrid({
  vaultId,
  files,
  selectedPaths,
  onToggleSelect,
  onPreview,
  onDownload,
  onDelete,
}: FileGridProps) {
  const selectionActive = (selectedPaths?.size ?? 0) > 0;

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
      {files.map((file) => (
        <FileItem
          key={file.path}
          vaultId={vaultId}
          file={file}
          isSelected={selectedPaths?.has(file.path) ?? false}
          selectionActive={selectionActive}
          onToggleSelect={
            onToggleSelect ? (shift) => onToggleSelect(file.path, shift) : undefined
          }
          onPreview={() => onPreview(file)}
          onDownload={() => onDownload(file.path)}
          onDelete={() => onDelete(file.path)}
        />
      ))}
    </div>
  );
}

interface FileItemProps {
  vaultId: string;
  file: FileEntry;
  isSelected: boolean;
  selectionActive: boolean;
  onToggleSelect?: (shift: boolean) => void;
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

function FileItem({
  vaultId,
  file,
  isSelected,
  selectionActive,
  onToggleSelect,
  onPreview,
  onDownload,
  onDelete,
}: FileItemProps) {
  const ext = getExt(file.path);
  const isImage = IMAGE_EXTS.has(ext);
  const isVideo = VIDEO_EXTS.has(ext);
  const sdk = useSdk();

  const { data: previewUrl } = useQuery({
    queryKey: ["preview", vaultId, file.path],
    queryFn: async () => {
      const blob = await sdk.files.download(vaultId, file.path);
      return URL.createObjectURL(blob);
    },
    enabled: isImage && Number(file.size) < 8 * 1024 * 1024,
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });

  const name = file.path.split("/").pop() ?? file.path;

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          className={`group relative flex flex-col gap-1 p-1.5 rounded-lg cursor-pointer transition-all bg-pop ring-1 ${
            isSelected
              ? "ring-primary bg-primary/5"
              : "ring-transparent hover:ring-border"
          }`}
          onClick={onPreview}
        >
          {/* Checkbox — always visible when selection is active; appears on hover otherwise */}
          {onToggleSelect && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onToggleSelect(e.shiftKey);
              }}
              className={`absolute top-2 left-2 z-10 flex items-center justify-center w-5 h-5 rounded border transition-opacity
                ${
                  selectionActive || isSelected
                    ? "opacity-100"
                    : "opacity-0 group-hover:opacity-100"
                }
                ${
                  isSelected
                    ? "bg-primary border-primary text-primary-foreground"
                    : "bg-background/80 border-border"
                }`}
              aria-label={isSelected ? "Deselect" : "Select"}
            >
              {isSelected && (
                <svg
                  viewBox="0 0 12 12"
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M2 6l3 3 5-5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              )}
            </button>
          )}

          {/* Hover delete button (right side) */}
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
