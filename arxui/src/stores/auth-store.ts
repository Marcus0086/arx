"use client";
import { create } from "zustand";

interface User {
  userId: string;
  email: string;
  tenantId: string;
}

interface AuthState {
  user: User | null;
  /** Whether we've attempted hydration from localStorage. */
  hydrated: boolean;
  setUser: (u: User | null) => void;
  setHydrated: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  hydrated: false,
  setUser: (u) => set({ user: u }),
  setHydrated: () => set({ hydrated: true }),
}));
