"use client";

import { SidebarTrigger } from "@/components/ui/sidebar";
import { Archive } from "lucide-react";

export function MobileHeader() {
  return (
    <header className="lg:hidden h-header-mobile sticky top-0 z-50 bg-background/80 backdrop-blur-sm border-b border-pop flex items-center px-4 gap-3">
      <SidebarTrigger />
      <div className="flex items-center gap-2">
        <Archive className="size-5 text-primary" />
        <span className="font-display text-xl tracking-tight">ARX DRIVE</span>
      </div>
    </header>
  );
}
