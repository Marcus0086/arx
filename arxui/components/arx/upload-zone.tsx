"use client";

import { useCallback, useState } from "react";
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
    noClick: true, // don't open picker on click (we have a button for that)
    noKeyboard: true,
    disabled,
  });

  return (
    <div {...getRootProps()} className="relative min-h-full">
      <input {...getInputProps()} />
      {children}

      {/* Drag overlay */}
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
