"use client";
import { create } from "zustand";

interface FileSelectionStore {
  vaultId: string | null;
  selected: Set<string>;
  anchor: string | null;
  setVault: (id: string) => void;
  toggle: (path: string) => void;
  selectRange: (allPaths: string[], target: string) => void;
  selectAll: (paths: string[]) => void;
  clear: () => void;
}

export const useFileSelectionStore = create<FileSelectionStore>((set, get) => ({
  vaultId: null,
  selected: new Set(),
  anchor: null,

  setVault: (id) => {
    if (get().vaultId !== id) {
      set({ vaultId: id, selected: new Set(), anchor: null });
    }
  },

  toggle: (path) =>
    set((s) => {
      const next = new Set(s.selected);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return { selected: next, anchor: path };
    }),

  selectRange: (allPaths, target) =>
    set((s) => {
      const anchor = s.anchor ?? target;
      const ai = allPaths.indexOf(anchor);
      const ti = allPaths.indexOf(target);
      const [start, end] = ai <= ti ? [ai, ti] : [ti, ai];
      const next = new Set(s.selected);
      for (const p of allPaths.slice(start, end + 1)) next.add(p);
      return { selected: next };
    }),

  selectAll: (paths) => set({ selected: new Set(paths) }),

  clear: () => set({ selected: new Set(), anchor: null }),
}));
