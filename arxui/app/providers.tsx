"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { useState } from "react";
import { SdkProvider } from "@/src/lib/sdk-context";

const ARX_URL = process.env.NEXT_PUBLIC_ARX_URL ?? "http://localhost:50051";

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            retry: 1,
          },
        },
      }),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <SdkProvider baseUrl={ARX_URL}>
        {children}
        <ReactQueryDevtools initialIsOpen={false} />
      </SdkProvider>
    </QueryClientProvider>
  );
}
