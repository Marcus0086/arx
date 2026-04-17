"use client";

import { SidebarProvider } from "@/components/ui/sidebar";
import { ArxSidebar } from "@/components/arx/arx-sidebar";
import { MobileHeader } from "@/components/arx/mobile-header";
import { AuthGuard } from "@/components/arx/auth-guard";
import { UploadQueuePanel } from "@/components/arx/upload-queue";
import { ClockWidget } from "@/components/arx/clock-widget";
import { StorageOverviewWidget } from "@/components/arx/storage-overview-widget";
import { RecentActivityWidget } from "@/components/arx/recent-activity-widget";

export default function VaultsLayout({ children }: { children: React.ReactNode }) {
  return (
    <AuthGuard>
      <SidebarProvider>
        <MobileHeader />
        <div className="w-full grid grid-cols-1 lg:grid-cols-12 gap-gap lg:px-sides">
          <div className="hidden lg:block col-span-2 top-0 relative">
            <ArxSidebar />
          </div>
          <div className="col-span-1 lg:col-span-7 py-sides">{children}</div>
          <div className="col-span-3 hidden lg:block">
            <div className="space-y-gap py-sides min-h-screen max-h-screen sticky top-0 overflow-clip">
              <ClockWidget />
              <StorageOverviewWidget />
              <RecentActivityWidget />
            </div>
          </div>
        </div>
        <UploadQueuePanel />
      </SidebarProvider>
    </AuthGuard>
  );
}
