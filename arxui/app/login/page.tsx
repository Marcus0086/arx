"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Bullet } from "@/components/ui/bullet";
import { Archive, Loader2, Lock } from "lucide-react";

function LoginForm() {
  const sdk = useSdk();
  const setUser = useAuthStore((s) => s.setUser);
  const router = useRouter();
  const searchParams = useSearchParams();
  const next = searchParams.get("next") ?? "/vaults";

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      await sdk.auth.login(email, password);
      const user = await sdk.auth.whoami();
      setUser(user);
      router.replace(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="bg-pop rounded-lg p-1.5 flex flex-col gap-1">
      {/* Header strip */}
      <div className="grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 pl-1 pr-1.5 py-2">
        <div className="flex items-center gap-2 text-xs uppercase text-muted-foreground">
          <Bullet />
          Sign In
        </div>
        <div className="flex items-center gap-2">
          <Lock className="w-4 h-4" />
          <span className="font-display text-lg uppercase">Authenticate</span>
        </div>
      </div>

      {/* Content */}
      <div className="p-3 py-3 rounded bg-card">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="email" className="text-xs uppercase">
              Email
            </Label>
            <Input
              id="email"
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              autoComplete="email"
              required
              disabled={loading}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="password" className="text-xs uppercase">
              Password
            </Label>
            <Input
              id="password"
              type="password"
              placeholder="••••••••"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
              required
              disabled={loading}
            />
          </div>

          {error && (
            <p className="text-xs uppercase text-destructive bg-destructive/10 border border-destructive/30 rounded px-3 py-2">
              {error}
            </p>
          )}

          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="animate-spin" />
                Signing in…
              </>
            ) : (
              "Sign in"
            )}
          </Button>
        </form>
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-background">
      <div className="w-full max-w-sm space-y-6">
        {/* Brand */}
        <div className="flex flex-col items-center gap-3 text-center">
          <div className="flex items-center justify-center size-12 rounded-lg bg-primary">
            <Archive className="size-6 opacity-80" />
          </div>
          <div>
            <h1 className="text-4xl font-display leading-none">ARX DRIVE</h1>
            <p className="text-xs uppercase text-muted-foreground mt-2 tracking-wide">
              Encrypted archive storage
            </p>
          </div>
        </div>

        <Suspense fallback={<div className="h-48 rounded-lg bg-pop animate-pulse" />}>
          <LoginForm />
        </Suspense>

        <p className="text-xs text-center uppercase text-muted-foreground/70 tracking-wide">
          Accounts are created by your administrator
        </p>
      </div>
    </div>
  );
}
