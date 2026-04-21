import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";

/**
 * Silently authenticates on mount using stored refresh token or credentials.
 * Never redirects to a login page — this is a local single-user app.
 */
export function AutoAuth({ children }: { children: React.ReactNode }) {
  const sdk = useSdk();
  const { user, hydrated, setUser, setHydrated } = useAuthStore();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (hydrated) return;
    authenticate();
  }, [hydrated]); // eslint-disable-line react-hooks/exhaustive-deps

  async function authenticate() {
    setError(null);
    try {
      // 1. Try JWT refresh (works if server kept the same DB since last run)
      const u = await sdk.auth.hydrate();
      if (u) {
        setUser(u);
        setHydrated();
        return;
      }

      // 2. Silent login with stored credentials
      const creds = await invoke<{ email: string; password: string } | null>(
        "get_credentials",
      ).catch(() => null);

      if (creds) {
        await sdk.auth.login(creds.email, creds.password);
        const me = await sdk.auth.whoami();
        if (me) {
          setUser(me);
          setHydrated();
          return;
        }
      }

      setUser(null);
      setHydrated();
      setError("Could not authenticate. The local database may have been reset.");
    } catch {
      setUser(null);
      setHydrated();
      setError("Could not connect to the local server. Try restarting ARX Drive.");
    }
  }

  if (!hydrated) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !user) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center gap-4 text-center p-8">
        <p className="text-sm text-muted-foreground max-w-sm">
          {error ?? "Authentication failed."}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            useAuthStore.getState().reset();
          }}
        >
          <RefreshCw className="size-4" />
          Retry
        </Button>
      </div>
    );
  }

  return <>{children}</>;
}
