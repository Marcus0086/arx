import { useEffect, useState } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { invoke } from "@tauri-apps/api/core";
import { Toaster } from "sonner";

import { SdkProvider } from "@/src/lib/sdk-context";
import { SetupGuard } from "@/components/arx/setup-guard";
import { AuthGuard } from "@/components/arx/auth-guard";

import SetupRoute from "@/src/routes/setup";
import LoginRoute from "@/src/routes/login";
import VaultsRoute from "@/src/routes/vaults";
import VaultDetailRoute from "@/src/routes/vault-detail";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30_000,
    },
  },
});

export default function App() {
  const [serverUrl, setServerUrl] = useState<string | null>(null);

  useEffect(() => {
    invoke<string>("get_server_url")
      .then(setServerUrl)
      .catch(() => {
        // Fallback for running outside Tauri (plain vite dev)
        setServerUrl(import.meta.env.VITE_ARX_URL ?? "http://localhost:50051");
      });
  }, []);

  if (!serverUrl) {
    return (
      <div className="h-screen flex items-center justify-center bg-background">
        <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
      <SdkProvider baseUrl={serverUrl}>
        <SetupGuard>
          <Routes>
            <Route path="/setup" element={<SetupRoute />} />
            <Route path="/login" element={<LoginRoute />} />
            <Route
              path="/vaults"
              element={
                <AuthGuard>
                  <VaultsRoute />
                </AuthGuard>
              }
            />
            <Route
              path="/vaults/:vaultId"
              element={
                <AuthGuard>
                  <VaultDetailRoute />
                </AuthGuard>
              }
            />
            <Route path="*" element={<Navigate to="/vaults" replace />} />
          </Routes>
        </SetupGuard>
        <Toaster richColors position="bottom-right" />
        {import.meta.env.DEV && <ReactQueryDevtools />}
      </SdkProvider>
    </QueryClientProvider>
  );
}
