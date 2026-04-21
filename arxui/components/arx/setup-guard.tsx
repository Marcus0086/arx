import { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";

export function SetupGuard({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const location = useLocation();
  const [checked, setChecked] = useState(false);

  useEffect(() => {
    invoke<boolean>("is_setup_complete")
      .then((complete) => {
        if (!complete && location.pathname !== "/setup") {
          navigate("/setup", { replace: true });
        }
        setChecked(true);
      })
      .catch(() => {
        // Running outside Tauri (e.g. pure vite dev) — skip setup
        setChecked(true);
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!checked) {
    return (
      <div className="h-screen flex items-center justify-center bg-background">
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return <>{children}</>;
}
