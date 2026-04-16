"use client";

import { useRouter, usePathname } from "next/navigation";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Archive, HardDrive, LogOut, ShieldCheck, User } from "lucide-react";
import { cn } from "@/lib/utils";

interface SidebarProps {
  user: { userId: string; email: string; tenantId: string };
}

const navItems = [
  { label: "My Vaults", href: "/vaults", icon: HardDrive },
  { label: "Verified", href: "/vaults?filter=verified", icon: ShieldCheck },
];

export function AppSidebar({ user }: SidebarProps) {
  const sdk = useSdk();
  const setUser = useAuthStore((s) => s.setUser);
  const router = useRouter();
  const pathname = usePathname();

  async function handleLogout() {
    await sdk.auth.logout();
    setUser(null);
    router.replace("/login");
  }

  return (
    <aside className="flex flex-col w-60 border-r border-border/50 bg-sidebar shrink-0">
      {/* Logo */}
      <div className="flex items-center gap-2.5 px-4 py-5 border-b border-border/50">
        <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-primary/10 border border-primary/20 shrink-0">
          <Archive className="w-4 h-4 text-primary" />
        </div>
        <span className="font-semibold text-sm tracking-tight">ARX Drive</span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-2 py-3 space-y-0.5">
        {navItems.map(({ label, href, icon: Icon }) => (
          <Button
            key={href}
            variant="ghost"
            className={cn(
              "w-full justify-start gap-2.5 h-9 px-2.5 text-sm font-medium",
              pathname === href.split("?")[0]
                ? "bg-accent text-accent-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
            onClick={() => router.push(href)}
          >
            <Icon className="w-4 h-4 shrink-0" />
            {label}
          </Button>
        ))}
      </nav>

      <Separator className="opacity-50" />

      {/* User */}
      <div className="px-3 py-3 space-y-1">
        <div className="flex items-center gap-2 px-2 py-1.5 rounded-md">
          <div className="flex items-center justify-center w-7 h-7 rounded-full bg-muted shrink-0">
            <User className="w-3.5 h-3.5 text-muted-foreground" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-xs font-medium truncate">{user.email}</p>
            <p className="text-[10px] text-muted-foreground truncate">
              {user.tenantId.slice(0, 8)}…
            </p>
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-start gap-2 h-8 px-2 text-xs text-muted-foreground hover:text-foreground"
          onClick={handleLogout}
        >
          <LogOut className="w-3.5 h-3.5" />
          Sign out
        </Button>
      </div>
    </aside>
  );
}
