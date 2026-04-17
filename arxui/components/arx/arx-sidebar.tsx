"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Bullet } from "@/components/ui/bullet";
import DotsVerticalIcon from "@/components/icons/dots-vertical";
import { Archive, HardDrive, LogOut } from "lucide-react";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";
import { UserAvatar } from "@/components/arx/user-avatar";

export function ArxSidebar() {
  const pathname = usePathname();
  const router = useRouter();
  const sdk = useSdk();
  const { user, setUser } = useAuthStore();

  async function handleLogout() {
    try {
      await sdk.auth.logout();
    } finally {
      setUser(null);
      router.replace("/login");
    }
  }

  const shortName = user?.email?.split("@")[0]?.toUpperCase() ?? "USER";

  return (
    <Sidebar className="py-sides">
      <SidebarHeader className="rounded-t-lg flex gap-3 flex-row rounded-b-none">
        <div className="flex overflow-clip size-12 shrink-0 items-center justify-center rounded bg-sidebar-primary-foreground/10 text-sidebar-primary-foreground">
          <Archive className="size-6" />
        </div>
        <div className="grid flex-1 text-left leading-tight">
          <span className="text-2xl font-display">ARX DRIVE</span>
          <span className="text-xs uppercase">Encrypted Storage</span>
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup className="rounded-t-none">
          <SidebarGroupLabel>
            <Bullet className="mr-2" />
            Storage
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton asChild isActive={pathname.startsWith("/vaults")}>
                  <Link href="/vaults">
                    <HardDrive className="size-5" />
                    <span>My Vaults</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="p-0">
        <SidebarGroup>
          <SidebarGroupLabel>
            <Bullet className="mr-2" />
            User
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <Popover>
                  <PopoverTrigger className="flex gap-0.5 w-full group cursor-pointer">
                    <UserAvatar seed={user?.userId ?? user?.email ?? "anon"} size={56} />
                    <div className="group/item pl-3 pr-1.5 pt-2 pb-1.5 flex-1 flex bg-sidebar-accent hover:bg-sidebar-accent-active/75 items-center rounded group-data-[state=open]:bg-sidebar-accent-active group-data-[state=open]:text-sidebar-accent-foreground">
                      <div className="grid flex-1 text-left leading-tight min-w-0">
                        <span className="truncate text-xl font-display">{shortName}</span>
                        <span className="truncate text-xs uppercase opacity-50 group-hover/item:opacity-100">
                          {user?.email ?? ""}
                        </span>
                      </div>
                      <DotsVerticalIcon className="ml-auto size-4" />
                    </div>
                  </PopoverTrigger>
                  <PopoverContent
                    className="w-56 p-0"
                    side="top"
                    align="end"
                    sideOffset={4}
                  >
                    <div className="flex flex-col">
                      <button
                        onClick={handleLogout}
                        className="flex items-center gap-2 px-4 py-2 text-sm hover:bg-accent text-left"
                      >
                        <LogOut className="size-4" />
                        Sign out
                      </button>
                    </div>
                  </PopoverContent>
                </Popover>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
