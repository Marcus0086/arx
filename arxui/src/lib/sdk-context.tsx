import { createContext, useContext, useMemo } from "react";
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
          window.location.replace("/login");
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
