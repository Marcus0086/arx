"use client";

import { useCallback, useState, useEffect, useRef } from "react";
import { useDropzone } from "react-dropzone";
import { Upload } from "lucide-react";
import { cn } from "@/lib/utils";

interface UploadZoneProps {
  children: React.ReactNode;
  onDrop: (files: File[]) => void;
  disabled?: boolean;
}

export function UploadZone({ children, onDrop, disabled }: UploadZoneProps) {
  const [isDragOver, setIsDragOver] = useState(false);

  // Keep stable refs so the effect closure doesn't go stale.
  const onDropRef = useRef(onDrop);
  const disabledRef = useRef(disabled);
  useEffect(() => {
    onDropRef.current = onDrop;
  }, [onDrop]);
  useEffect(() => {
    disabledRef.current = disabled;
  }, [disabled]);

  // Tauri-native OS file drag-drop (handles cases where HTML5 DnD doesn't fire
  // in WKWebView for files dragged from the macOS Finder).
  useEffect(() => {
    let cleanup: (() => void) | undefined;

    async function setup() {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      const { readFile } = await import("@tauri-apps/plugin-fs");

      cleanup = await getCurrentWebview().onDragDropEvent(async (event) => {
        if (disabledRef.current) return;
        const p = event.payload;

        if (p.type === "enter") {
          setIsDragOver(true);
        } else if (p.type === "leave") {
          setIsDragOver(false);
        } else if (p.type === "drop") {
          setIsDragOver(false);
          const files = await Promise.all(
            p.paths.map(async (filePath) => {
              const bytes = await readFile(filePath);
              const name = filePath.split(/[/\\]/).pop() ?? filePath;
              return new File([bytes], name);
            }),
          );
          if (files.length > 0) onDropRef.current(files);
        }
      });
    }

    setup().catch(() => {
      // Not running inside Tauri — rely on HTML5 DnD via react-dropzone below.
    });

    return () => {
      cleanup?.();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleDrop = useCallback(
    (acceptedFiles: File[]) => {
      setIsDragOver(false);
      if (acceptedFiles.length > 0) onDrop(acceptedFiles);
    },
    [onDrop],
  );

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop: handleDrop,
    onDragEnter: () => setIsDragOver(true),
    onDragLeave: () => setIsDragOver(false),
    noClick: true,
    noKeyboard: true,
    disabled,
  });

  return (
    <div {...getRootProps()} className="relative min-h-full">
      <input {...getInputProps()} />
      {children}

      {(isDragActive || isDragOver) && (
        <div
          className={cn(
            "absolute inset-0 z-50 flex flex-col items-center justify-center gap-3",
            "bg-background/90 backdrop-blur-sm border-2 border-dashed border-primary rounded-xl m-2",
          )}
        >
          <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-primary/10 border border-primary/20">
            <Upload className="w-8 h-8 text-primary" />
          </div>
          <div className="text-center">
            <p className="font-semibold text-lg">Drop to upload</p>
            <p className="text-sm text-muted-foreground">
              Files will be added to this vault
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
