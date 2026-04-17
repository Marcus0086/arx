"use client";

import React, { useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import TVNoise from "@/components/ui/tv-noise";

export function ClockWidget() {
  const [time, setTime] = useState<Date | null>(null);

  useEffect(() => {
    setTime(new Date());
    const id = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const timeStr = time
    ? time.toLocaleTimeString("en-US", {
        hour12: true,
        hour: "numeric",
        minute: "2-digit",
      })
    : "--:--";
  const dayOfWeek = time ? time.toLocaleDateString("en-US", { weekday: "long" }) : "";
  const restOfDate = time
    ? time.toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" })
    : "";
  const tz = time
    ? (new Intl.DateTimeFormat("en-US", { timeZoneName: "short" })
        .formatToParts(time)
        .find((p) => p.type === "timeZoneName")?.value ?? "")
    : "";

  return (
    <Card className="w-full aspect-[2] relative overflow-hidden">
      <TVNoise opacity={0.3} intensity={0.2} speed={40} />
      <CardContent className="bg-accent/30 flex-1 flex flex-col justify-between text-sm font-medium uppercase relative z-20">
        <div className="flex justify-between items-center">
          <span className="opacity-50">{dayOfWeek}</span>
          <span>{restOfDate}</span>
        </div>
        <div className="text-center">
          <div className="text-5xl font-display" suppressHydrationWarning>
            {timeStr}
          </div>
        </div>
        <div className="flex justify-between items-center">
          <span className="opacity-50">ARX</span>
          <span>Drive</span>
          <Badge variant="secondary" className="bg-accent">
            {tz}
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}
