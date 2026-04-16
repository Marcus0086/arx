"use client";
import { create } from "zustand";
import { nanoid } from "nanoid";

export interface UploadItem {
  id: string;
  fileName: string;
  fileSize: number;
  vaultId: string;
  status: "queued" | "uploading" | "done" | "error";
  bytesUploaded: number;
  error?: string;
}

interface UploadState {
  items: UploadItem[];
  add: (files: File[], vaultId: string) => UploadItem[];
  update: (id: string, patch: Partial<UploadItem>) => void;
  remove: (id: string) => void;
  clearDone: () => void;
}

export const useUploadStore = create<UploadState>((set, get) => ({
  items: [],
  add: (files, vaultId) => {
    const newItems: UploadItem[] = files.map((f) => ({
      id: nanoid(),
      fileName: f.name,
      fileSize: f.size,
      vaultId,
      status: "queued",
      bytesUploaded: 0,
    }));
    set((s) => ({ items: [...s.items, ...newItems] }));
    return newItems;
  },
  update: (id, patch) =>
    set((s) => ({
      items: s.items.map((it) => (it.id === id ? { ...it, ...patch } : it)),
    })),
  remove: (id) => set((s) => ({ items: s.items.filter((it) => it.id !== id) })),
  clearDone: () =>
    set((s) => ({ items: s.items.filter((it) => it.status !== "done") })),
}));
