"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";
import { AppSidebar } from "./app-sidebar";
import { UploadQueuePanel } from "./upload-queue";
import { Loader2 } from "lucide-react";

export function AppShell({ children }: { children: React.ReactNode }) {
  const sdk = useSdk();
  const { user, hydrated, setUser, setHydrated } = useAuthStore();
  const router = useRouter();

  useEffect(() => {
    if (hydrated) return;
    sdk.auth.hydrate().then((u) => {
      setUser(u);
      setHydrated();
      if (!u) router.replace("/login");
    });
  }, [hydrated, sdk, setUser, setHydrated, router]);

  if (!hydrated) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!user) return null;

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <AppSidebar user={user} />
      <main className="flex-1 overflow-auto">{children}</main>
      <UploadQueuePanel />
    </div>
  );
}
