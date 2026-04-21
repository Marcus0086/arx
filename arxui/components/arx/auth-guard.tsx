import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const sdk = useSdk();
  const { user, hydrated, setUser, setHydrated } = useAuthStore();
  const navigate = useNavigate();

  useEffect(() => {
    if (hydrated) return;
    sdk.auth.hydrate().then((u) => {
      setUser(u);
      setHydrated();
      if (!u) navigate("/login", { replace: true });
    });
  }, [hydrated, sdk, setUser, setHydrated, navigate]);

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
