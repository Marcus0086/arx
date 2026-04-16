import { jwtDecode } from "jwt-decode";

export interface TokenStore {
  getAccessToken(): string | null;
  setAccessToken(token: string): void;
  getRefreshToken(): string | null;
  setRefreshToken(token: string): void;
  clear(): void;
}

export function isTokenExpired(token: string): boolean {
  try {
    const { exp } = jwtDecode<{ exp: number }>(token);
    // Consider expired 30 seconds early to avoid races
    return Date.now() / 1000 > exp - 30;
  } catch {
    return true;
  }
}

/** In-memory store — access token lives here (intentionally lost on page refresh). */
class MemoryStore {
  private accessToken: string | null = null;
  getAccessToken() {
    return this.accessToken;
  }
  setAccessToken(t: string) {
    this.accessToken = t;
  }
}

const _mem = new MemoryStore();

/** Default browser token store: access token in memory, refresh token in localStorage. */
export const defaultTokenStore: TokenStore = {
  getAccessToken: () => _mem.getAccessToken(),
  setAccessToken: (t) => _mem.setAccessToken(t),
  getRefreshToken: () => {
    if (typeof window === "undefined") return null;
    return localStorage.getItem("arx_refresh_token");
  },
  setRefreshToken: (t) => {
    if (typeof window !== "undefined") localStorage.setItem("arx_refresh_token", t);
  },
  clear: () => {
    _mem.setAccessToken("");
    if (typeof window !== "undefined") localStorage.removeItem("arx_refresh_token");
  },
};
