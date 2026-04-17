import { createClient, type PromiseClient } from "@connectrpc/connect";
import { ArxService } from "@/src/gen/arx_connect";
import type { TokenStore } from "./token-store";
import type { Transport } from "@connectrpc/connect";

export interface LoginResult {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

export interface WhoamiResult {
  userId: string;
  email: string;
  tenantId: string;
}

type AuthChangeCallback = (authenticated: boolean) => void;

export class AuthService {
  private client: PromiseClient<typeof ArxService>;
  private store: TokenStore;
  private listeners: Set<AuthChangeCallback> = new Set();

  constructor(transport: Transport, store: TokenStore) {
    this.client = createClient(ArxService, transport);
    this.store = store;
  }

  async login(email: string, password: string): Promise<LoginResult> {
    const res = await this.client.login({ email, password });
    if (res.error) throw new Error(res.error);
    this.store.setAccessToken(res.accessToken);
    this.store.setRefreshToken(res.refreshToken);
    // Set presence cookie so middleware can check auth server-side
    if (typeof document !== "undefined") {
      document.cookie = `arx_session=1; path=/; max-age=${60 * 60 * 24 * 30}; SameSite=Lax`;
    }
    this.notify(true);
    return {
      accessToken: res.accessToken,
      refreshToken: res.refreshToken,
      expiresIn: res.expiresIn,
    };
  }

  async logout(): Promise<void> {
    const rt = this.store.getRefreshToken();
    if (rt) {
      try {
        await this.client.logout({ refreshToken: rt });
      } catch {
        /* ignore */
      }
    }
    this.store.clear();
    // Clear presence cookie
    if (typeof document !== "undefined") {
      document.cookie = "arx_session=; path=/; max-age=0";
    }
    this.notify(false);
  }

  async whoami(): Promise<WhoamiResult> {
    const res = await this.client.whoami({});
    return { userId: res.userId, email: res.email, tenantId: res.tenantId };
  }

  isAuthenticated(): boolean {
    return !!this.store.getRefreshToken();
  }

  /**
   * On page load, try to restore a session using the stored refresh token.
   * Returns the user profile on success, or null if no valid session exists.
   */
  async hydrate(): Promise<WhoamiResult | null> {
    const rt = this.store.getRefreshToken();
    if (!rt) return null;
    try {
      const res = await this.client.refreshToken({ refreshToken: rt });
      if (res.error) throw new Error(res.error);
      this.store.setAccessToken(res.accessToken);
      this.store.setRefreshToken(res.newRefreshToken);
      if (typeof document !== "undefined") {
        document.cookie = `arx_session=1; path=/; max-age=${60 * 60 * 24 * 30}; SameSite=Lax`;
      }
      return this.whoami();
    } catch {
      this.store.clear();
      if (typeof document !== "undefined") {
        document.cookie = "arx_session=; path=/; max-age=0";
      }
      return null;
    }
  }

  /** Subscribe to auth state changes. Returns unsubscribe function. */
  onAuthChange(cb: AuthChangeCallback): () => void {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  private notify(authenticated: boolean) {
    this.listeners.forEach((cb) => cb(authenticated));
  }
}
