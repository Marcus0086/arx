import { createContext, useContext, useMemo } from "react";
import { useAuthStore } from "@/src/stores/auth-store";
import { ArxClient } from "@/src/sdk";

const SdkContext = createContext<ArxClient | null>(null);

export function SdkProvider({
  children,
  baseUrl,
}: {
  children: React.ReactNode;
  baseUrl: string;
}) {
  const client = useMemo(
    () =>
      ArxClient.create({
        baseUrl,
        onAuthExpired: () => {
          // Reset auth state — AuthGuard will re-run and attempt auto-login
          // with stored credentials before falling back to the login screen.
          useAuthStore.getState().reset();
        },
      }),
    [baseUrl],
  );

  return <SdkContext.Provider value={client}>{children}</SdkContext.Provider>;
}

export function useSdk(): ArxClient {
  const sdk = useContext(SdkContext);
  if (!sdk) throw new Error("useSdk must be used within SdkProvider");
  return sdk;
}
