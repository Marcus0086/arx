"use client";

export default function RootError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-4 p-8 text-center">
      <p className="font-display text-2xl uppercase">Something went wrong</p>
      <p className="text-sm text-muted-foreground max-w-sm">{error.message}</p>
      <button
        onClick={reset}
        className="text-xs uppercase tracking-wide border border-border px-4 py-2 rounded hover:bg-pop transition-colors"
      >
        Try again
      </button>
    </div>
  );
}
