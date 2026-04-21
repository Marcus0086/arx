import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
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
import { Bullet } from "@/components/ui/bullet";
import { Archive, HardDrive } from "lucide-react";
import { useAuthStore } from "@/src/stores/auth-store";
import { UserAvatar } from "@/components/arx/user-avatar";

export function ArxSidebar() {
  const { pathname } = useLocation();
  const [rootDir, setRootDir] = useState<string>("");
  const { user } = useAuthStore();

  useEffect(() => {
    invoke<string>("get_root_dir")
      .then(setRootDir)
      .catch(() => {});
  }, []);

  const shortName = user?.email?.split("@")[0]?.toUpperCase() ?? "LOCAL";

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
                  <Link to="/vaults">
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
            Local User
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <div className="flex gap-0.5 w-full">
                  <UserAvatar seed={user?.userId ?? user?.email ?? "local"} size={56} />
                  <div className="pl-3 pr-1.5 pt-2 pb-1.5 flex-1 flex bg-sidebar-accent items-center rounded">
                    <div className="grid flex-1 text-left leading-tight min-w-0">
                      <span className="truncate text-xl font-display">{shortName}</span>
                      <span className="truncate text-xs uppercase opacity-50">
                        {rootDir ? rootDir.split("/").pop() || rootDir : "ARX Drive"}
                      </span>
                    </div>
                  </div>
                </div>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        {rootDir && (
          <div className="px-3 pb-2 pt-1">
            <div
              className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60 cursor-default"
              title={rootDir}
            >
              <HardDrive className="h-3 w-3 shrink-0" />
              <span className="truncate font-mono">
                {rootDir.split("/").pop() || rootDir}
              </span>
            </div>
          </div>
        )}
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
