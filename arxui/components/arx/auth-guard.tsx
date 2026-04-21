import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const sdk = useSdk();
  const { user, hydrated, setUser, setHydrated } = useAuthStore();
  const navigate = useNavigate();

  useEffect(() => {
    if (hydrated) return;

    async function authenticate() {
      // 1. Try JWT refresh (works if server kept the same DB)
      const u = await sdk.auth.hydrate();
      if (u) {
        setUser(u);
        setHydrated();
        return;
      }

      // 2. Try silent auto-login with stored credentials (survives server restart)
      try {
        const creds = await invoke<{ email: string; password: string } | null>(
          "get_credentials",
        );
        if (creds) {
          await sdk.auth.login(creds.email, creds.password);
          const me = await sdk.auth.whoami();
          if (me) {
            setUser(me);
            setHydrated();
            return;
          }
        }
      } catch {
        // credentials invalid or server not ready yet — fall through to login
      }

      // 3. No valid session — go to login
      setUser(null);
      setHydrated();
      navigate("/login", { replace: true });
    }

    authenticate();
  }, [hydrated]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!hydrated) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }
  if (!user) return null;
  return <>{children}</>;
}
