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

const REFRESH_COOKIE = "arx_refresh_token";
const REFRESH_STORAGE_KEY = "arx_refresh_token";
const REFRESH_MAX_AGE_SECS = 60 * 60 * 24 * 30; // 30 days

function readCookie(name: string): string | null {
  if (typeof document === "undefined") return null;
  const prefix = `${name}=`;
  for (const part of document.cookie.split(";")) {
    const p = part.trim();
    if (p.startsWith(prefix)) {
      return decodeURIComponent(p.slice(prefix.length));
    }
  }
  return null;
}

function writeCookie(name: string, value: string, maxAgeSecs: number) {
  if (typeof document === "undefined") return;
  const secure =
    typeof window !== "undefined" && window.location.protocol === "https:"
      ? "; Secure"
      : "";
  document.cookie =
    `${name}=${encodeURIComponent(value)}; path=/; max-age=${maxAgeSecs}; SameSite=Lax` +
    secure;
}

function clearCookie(name: string) {
  if (typeof document === "undefined") return;
  document.cookie = `${name}=; path=/; max-age=0; SameSite=Lax`;
}

/**
 * Default browser token store.
 *
 * - Access token: in-memory only — a short-lived JWT. Not persisted so XSS
 *   steals a token that expires in 15m instead of 30d.
 * - Refresh token: persisted in BOTH localStorage and a cookie (SameSite=Lax,
 *   30-day max-age). Reads prefer localStorage; cookie is a fallback for
 *   private-browsing contexts and Safari's localStorage pruning. Either alone
 *   is enough to restore a session on reload.
 */
export const defaultTokenStore: TokenStore = {
  getAccessToken: () => _mem.getAccessToken(),
  setAccessToken: (t) => _mem.setAccessToken(t),
  getRefreshToken: () => {
    if (typeof window === "undefined") return null;
    try {
      const ls = localStorage.getItem(REFRESH_STORAGE_KEY);
      if (ls) return ls;
    } catch {
      /* localStorage may throw in private mode / with cookies disabled */
    }
    return readCookie(REFRESH_COOKIE);
  },
  setRefreshToken: (t) => {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem(REFRESH_STORAGE_KEY, t);
    } catch {
      /* ignore — cookie fallback still works */
    }
    writeCookie(REFRESH_COOKIE, t, REFRESH_MAX_AGE_SECS);
  },
  clear: () => {
    _mem.setAccessToken("");
    if (typeof window !== "undefined") {
      try {
        localStorage.removeItem(REFRESH_STORAGE_KEY);
      } catch {
        /* ignore */
      }
      clearCookie(REFRESH_COOKIE);
    }
  },
};
