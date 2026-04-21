import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Archive,
  ChevronRight,
  FolderOpen,
  HardDrive,
  Loader2,
  Lock,
} from "lucide-react";
import { AdminService } from "@/src/sdk/admin-service";
import { useSdk } from "@/src/lib/sdk-context";
import { useAuthStore } from "@/src/stores/auth-store";

const storageSchema = z.object({
  rootDir: z.string().min(1, "Choose a storage location"),
});

type StorageForm = z.infer<typeof storageSchema>;

const STEPS = ["Welcome", "Storage", "Setting up"] as const;

function StepDot({ active, done }: { active: boolean; done: boolean }) {
  return (
    <div
      className={`w-2 h-2 rounded-full transition-colors ${
        done ? "bg-primary" : active ? "bg-primary/60" : "bg-muted-foreground/30"
      }`}
    />
  );
}

export default function SetupPage() {
  const navigate = useNavigate();
  const sdk = useSdk();
  const { setUser, setHydrated } = useAuthStore();
  const [step, setStep] = useState(0);
  const [rootDir, setRootDir] = useState("");
  const [setupError, setSetupError] = useState<string | null>(null);

  const storageForm = useForm<StorageForm>({
    resolver: zodResolver(storageSchema),
    defaultValues: { rootDir: "" },
  });

  useEffect(() => {
    invoke<string>("get_root_dir")
      .then((dir) => {
        if (dir) {
          setRootDir(dir);
          storageForm.setValue("rootDir", dir);
        }
      })
      .catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  async function handlePickFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked) {
        setRootDir(picked);
        storageForm.setValue("rootDir", picked, { shouldValidate: true });
      }
    } catch {
      // Running outside Tauri — ignore
    }
  }

  async function handleFinish(data: StorageForm) {
    setStep(2);
    setSetupError(null);

    try {
      // 1. Save root dir
      const effectiveRootDir =
        rootDir || (await invoke<string>("get_root_dir").catch(() => ""));
      if (!effectiveRootDir) throw new Error("Storage location is required");
      await invoke("save_root_dir", { rootDir: effectiveRootDir });

      // 2. Wait for gRPC server to be ready
      const serverUrl = await invoke<string>("get_server_url").catch(
        () => "http://localhost:50051",
      );
      await waitForServer(serverUrl);

      // 3. Get admin key
      const adminKey = await invoke<string>("get_admin_key").catch(() => "");

      // 4. Create tenant + local user
      const admin = new AdminService(serverUrl, adminKey);
      const tenantId = await admin.createTenant(`workspace-${Date.now()}`);

      const email = "local@arx.local";
      let password: string = crypto.randomUUID();

      try {
        await admin.createUser(tenantId, email, password);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (msg.toLowerCase().includes("unique")) {
          // User already exists from a prior setup run — reuse stored credentials
          const stored = await invoke<{ email: string; password: string } | null>(
            "get_credentials",
          ).catch(() => null);
          if (stored?.email === email) {
            password = stored.password;
          } else {
            throw new Error(
              "Local account exists but credentials are missing. Delete the ARX data directory and relaunch to start fresh.",
            );
          }
        } else {
          throw err;
        }
      }

      // 5. Auto-login so the session is ready immediately
      await sdk.auth.login(email, password);
      const me = await sdk.auth.whoami();
      if (me) {
        setUser(me);
        setHydrated();
      }

      // 6. Persist credentials for silent re-login on restart
      await invoke("save_credentials", { email, password }).catch(() => {});

      // 7. Mark setup complete and open the app
      await invoke("mark_setup_complete");
      navigate("/vaults", { replace: true });
    } catch (err) {
      setSetupError(
        err instanceof Error ? err.message : "Setup failed. Please try again.",
      );
      setStep(1);
    }
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-4 bg-background">
      <div className="w-full max-w-md space-y-8">
        <div className="flex flex-col items-center gap-3 text-center">
          <div className="flex items-center justify-center size-14 rounded-2xl bg-primary shadow-lg">
            <Archive className="size-7 opacity-90" />
          </div>
          <div>
            <h1 className="text-4xl font-display leading-none">ARX DRIVE</h1>
            <p className="text-xs uppercase text-muted-foreground mt-2 tracking-wide">
              Encrypted archive storage
            </p>
          </div>
        </div>

        <div className="flex items-center justify-center gap-2">
          {STEPS.map((_, i) => (
            <StepDot key={i} active={i === step} done={i < step} />
          ))}
        </div>

        <div className="bg-pop rounded-lg p-1.5">
          <div className="flex items-center gap-2 text-xs uppercase text-muted-foreground px-2 py-2">
            <div className="w-1.5 h-1.5 rounded-full bg-primary" />
            {STEPS[step]}
          </div>

          <div className="rounded bg-card p-5">
            {step === 0 && <WelcomeStep onNext={() => setStep(1)} />}
            {step === 1 && (
              <StorageStep
                form={storageForm}
                rootDir={rootDir}
                error={setupError}
                onPickFolder={handlePickFolder}
                onNext={(data) => {
                  setRootDir(data.rootDir);
                  handleFinish(data);
                }}
              />
            )}
            {step === 2 && <SettingUpStep />}
          </div>
        </div>
      </div>
    </div>
  );
}

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="font-display text-2xl uppercase">Welcome</h2>
        <p className="text-sm text-muted-foreground leading-relaxed">
          ARX Drive stores your files as encrypted, compressed archives on your local
          drive. No cloud account required — your data stays on your machine.
        </p>
      </div>
      <div className="space-y-2 text-sm">
        {[
          { icon: Lock, text: "End-to-end encrypted with XChaCha20-Poly1305" },
          { icon: HardDrive, text: "Files stored locally — no cloud dependency" },
          { icon: Archive, text: "Efficient compression with content-defined chunking" },
        ].map(({ icon: Icon, text }) => (
          <div key={text} className="flex items-center gap-2.5 text-muted-foreground">
            <Icon className="w-3.5 h-3.5 shrink-0 text-primary" />
            <span>{text}</span>
          </div>
        ))}
      </div>
      <Button className="w-full" onClick={onNext}>
        Get started
        <ChevronRight />
      </Button>
    </div>
  );
}

function StorageStep({
  form,
  rootDir,
  error,
  onPickFolder,
  onNext,
}: {
  form: ReturnType<typeof useForm<StorageForm>>;
  rootDir: string;
  error: string | null;
  onPickFolder: () => void;
  onNext: (data: StorageForm) => void;
}) {
  return (
    <form onSubmit={form.handleSubmit(onNext)} className="space-y-5">
      <div className="space-y-1.5">
        <h2 className="font-display text-2xl uppercase">Storage location</h2>
        <p className="text-sm text-muted-foreground">
          Choose where ARX Drive stores your encrypted archive files.
        </p>
      </div>

      <div className="space-y-2">
        <Label className="text-xs uppercase">Location</Label>
        <div className="flex gap-2">
          <Input
            {...form.register("rootDir")}
            value={rootDir}
            readOnly
            placeholder="No location selected"
            className="font-mono text-xs flex-1"
          />
          <Button
            type="button"
            variant="outline"
            onClick={onPickFolder}
            className="shrink-0"
          >
            <FolderOpen />
            Browse
          </Button>
        </div>
        {form.formState.errors.rootDir && (
          <p className="text-xs text-destructive">
            {form.formState.errors.rootDir.message}
          </p>
        )}
      </div>

      {error && (
        <p className="text-xs text-destructive bg-destructive/10 border border-destructive/30 rounded px-3 py-2">
          {error}
        </p>
      )}

      <Button type="submit" className="w-full">
        Continue
        <ChevronRight />
      </Button>
    </form>
  );
}

function SettingUpStep() {
  return (
    <div className="flex flex-col items-center gap-5 py-6">
      <Loader2 className="w-8 h-8 animate-spin text-primary" />
      <div className="text-center space-y-1">
        <p className="font-display text-xl uppercase">Setting up…</p>
        <p className="text-xs text-muted-foreground uppercase tracking-wide">
          Starting server and initializing your drive
        </p>
      </div>
    </div>
  );
}

async function waitForServer(baseUrl: string, maxMs = 10_000): Promise<void> {
  const deadline = Date.now() + maxMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(1000) });
      if (res.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 500));
  }
}
