import { createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { ArxService } from "@/src/gen/arx_connect";
import { defaultTokenStore, isTokenExpired, type TokenStore } from "./token-store";

let _refreshPromise: Promise<string> | null = null;

/**
 * Build the gRPC-web transport with an auth interceptor.
 * The interceptor auto-refreshes the access token before each request.
 */
export function buildTransport(
  baseUrl: string,
  store: TokenStore,
  onRefreshFailed: () => void,
) {
  const authInterceptor: Interceptor = (next) => async (req) => {
    // Skip auth header injection for unauthenticated endpoints
    const unauthenticated = [
      "/arx.ArxService/Login",
      "/arx.ArxService/RefreshToken",
      "/arx.ArxService/Logout",
    ];
    if (!unauthenticated.some((p) => req.url.endsWith(p))) {
      let token = store.getAccessToken();
      if (!token || isTokenExpired(token)) {
        // Coalesce concurrent refreshes into a single request
        if (!_refreshPromise) {
          _refreshPromise = doRefresh(baseUrl, store, onRefreshFailed).finally(() => {
            _refreshPromise = null;
          });
        }
        token = await _refreshPromise;
      }
      if (token) req.header.set("authorization", `Bearer ${token}`);
    }
    return next(req);
  };

  return createGrpcWebTransport({
    baseUrl,
    interceptors: [authInterceptor],
  });
}

async function doRefresh(
  baseUrl: string,
  store: TokenStore,
  onFailed: () => void,
): Promise<string> {
  const refreshToken = store.getRefreshToken();
  if (!refreshToken) {
    onFailed();
    throw new Error("No refresh token");
  }
  // Use a bare transport (no interceptor) to avoid recursion
  const bare = createGrpcWebTransport({ baseUrl });
  const client = createClient(ArxService, bare);
  try {
    const res = await client.refreshToken({ refreshToken });
    store.setAccessToken(res.accessToken);
    store.setRefreshToken(res.newRefreshToken);
    return res.accessToken;
  } catch {
    store.clear();
    onFailed();
    throw new Error("Token refresh failed");
  }
}
