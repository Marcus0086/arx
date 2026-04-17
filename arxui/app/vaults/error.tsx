"use client";

export default function VaultsError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className="flex flex-col items-center justify-center py-20 gap-4 text-center">
      <p className="font-display text-xl uppercase">Failed to load vaults</p>
      <p className="text-xs text-muted-foreground">{error.message}</p>
      <button
        onClick={reset}
        className="text-xs uppercase tracking-wide border border-border px-3 py-1.5 rounded hover:bg-pop transition-colors"
      >
        Retry
      </button>
    </div>
  );
}
