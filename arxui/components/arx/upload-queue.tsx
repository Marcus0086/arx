"use client";

import { useUploadStore } from "@/src/stores/upload-store";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { CheckCircle2, Loader2, X, XCircle } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";

export function UploadQueuePanel() {
  const { items, clearDone, remove } = useUploadStore();

  if (items.length === 0) return null;

  const uploading = items.filter((i) => i.status === "uploading").length;
  const done = items.filter((i) => i.status === "done").length;

  return (
    <motion.div
      initial={{ y: 80, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      exit={{ y: 80, opacity: 0 }}
      className="fixed bottom-4 right-4 w-80 bg-card border border-border/50 rounded-xl shadow-2xl overflow-hidden z-50"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
        <div className="flex items-center gap-2">
          {uploading > 0 ? (
            <Loader2 className="w-4 h-4 animate-spin text-primary" />
          ) : (
            <CheckCircle2 className="w-4 h-4 text-green-500" />
          )}
          <span className="text-sm font-medium">
            {uploading > 0
              ? `Uploading ${uploading} file${uploading > 1 ? "s" : ""}…`
              : `${done} upload${done > 1 ? "s" : ""} complete`}
          </span>
        </div>
        <Button variant="ghost" size="icon" className="h-6 w-6" onClick={clearDone}>
          <X className="w-3.5 h-3.5" />
        </Button>
      </div>

      {/* List */}
      <div className="max-h-60 overflow-y-auto">
        <AnimatePresence initial={false}>
          {items.map((item) => (
            <motion.div
              key={item.id}
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="px-4 py-2.5 border-b border-border/30 last:border-0"
            >
              <div className="flex items-center gap-2 mb-1.5">
                <span className="text-xs font-medium truncate flex-1">{item.fileName}</span>
                <span className="text-[10px] text-muted-foreground shrink-0">
                  {formatBytes(item.fileSize)}
                </span>
                {item.status === "error" && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-4 w-4 shrink-0"
                    onClick={() => remove(item.id)}
                  >
                    <X className="w-2.5 h-2.5" />
                  </Button>
                )}
              </div>

              {item.status === "uploading" && (
                <Progress
                  value={(item.bytesUploaded / item.fileSize) * 100}
                  className="h-1"
                />
              )}
              {item.status === "done" && (
                <div className="flex items-center gap-1 text-green-500">
                  <CheckCircle2 className="w-3 h-3" />
                  <span className="text-[10px]">Done</span>
                </div>
              )}
              {item.status === "error" && (
                <div className="flex items-center gap-1 text-destructive">
                  <XCircle className="w-3 h-3" />
                  <span className="text-[10px]">{item.error ?? "Failed"}</span>
                </div>
              )}
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(1)} GB`;
}
