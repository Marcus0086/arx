import { create } from "zustand";

interface User {
  userId: string;
  email: string;
  tenantId: string;
}

interface AuthState {
  user: User | null;
  hydrated: boolean;
  setUser: (u: User | null) => void;
  setHydrated: () => void;
  /** Reset to unauthenticated — used by onAuthExpired to trigger re-auth. */
  reset: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  hydrated: false,
  setUser: (u) => set({ user: u }),
  setHydrated: () => set({ hydrated: true }),
  reset: () => set({ user: null, hydrated: false }),
}));
