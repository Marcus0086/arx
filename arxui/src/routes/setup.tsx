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
  Check,
  ChevronRight,
  FolderOpen,
  HardDrive,
  Loader2,
  Lock,
} from "lucide-react";
import { AdminService } from "@/src/sdk/admin-service";

// ── Zod schemas ────────────────────────────────────────────────────────────────

const storageSchema = z.object({
  rootDir: z.string().min(1, "Choose a storage location"),
});

const accountSchema = z
  .object({
    email: z.string().email("Invalid email address"),
    password: z.string().min(8, "Password must be at least 8 characters"),
    confirm: z.string(),
  })
  .refine((d) => d.password === d.confirm, {
    message: "Passwords do not match",
    path: ["confirm"],
  });

type StorageForm = z.infer<typeof storageSchema>;
type AccountForm = z.infer<typeof accountSchema>;

// ── Step indicators ────────────────────────────────────────────────────────────

const STEPS = ["Welcome", "Storage", "Account", "Setting up"] as const;

function StepDot({ active, done }: { active: boolean; done: boolean }) {
  return (
    <div
      className={`w-2 h-2 rounded-full transition-colors ${
        done ? "bg-primary" : active ? "bg-primary/60" : "bg-muted-foreground/30"
      }`}
    />
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export default function SetupPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState(0);
  const [rootDir, setRootDir] = useState("");
  const [setupError, setSetupError] = useState<string | null>(null);

  const storageForm = useForm<StorageForm>({
    resolver: zodResolver(storageSchema),
    defaultValues: { rootDir: "" },
  });

  const accountForm = useForm<AccountForm>({
    resolver: zodResolver(accountSchema),
    defaultValues: { email: "", password: "", confirm: "" },
  });

  // Load default rootDir from Tauri on mount
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

  async function handleFinish(data: AccountForm) {
    setStep(3);
    setSetupError(null);

    try {
      // 1. Save root dir to config (use Tauri's default if somehow empty)
      const effectiveRootDir =
        rootDir || (await invoke<string>("get_root_dir").catch(() => ""));
      if (!effectiveRootDir) throw new Error("Storage location is required");
      await invoke("save_root_dir", { rootDir: effectiveRootDir });

      // 2. Wait for gRPC server to be ready (poll up to 10s)
      const serverUrl = await invoke<string>("get_server_url").catch(
        () => "http://localhost:50051",
      );
      await waitForServer(serverUrl);

      // 3. Get admin key
      const adminKey = await invoke<string>("get_admin_key").catch(() => "");

      // 4. Create tenant + user via admin API
      const admin = new AdminService(serverUrl, adminKey);
      // Use a unique name so repeated setup runs never hit UNIQUE constraint
      const tenantId = await admin.createTenant(`workspace-${Date.now()}`);
      await admin.createUser(tenantId, data.email, data.password);

      // 5. Mark setup complete
      await invoke("mark_setup_complete");

      // 6. Navigate to login
      navigate("/login", { replace: true });
    } catch (err) {
      setSetupError(
        err instanceof Error ? err.message : "Setup failed. Please try again.",
      );
      setStep(2);
    }
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-4 bg-background">
      <div className="w-full max-w-md space-y-8">
        {/* Brand header */}
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

        {/* Step dots */}
        <div className="flex items-center justify-center gap-2">
          {STEPS.map((_, i) => (
            <StepDot key={i} active={i === step} done={i < step} />
          ))}
        </div>

        {/* Step content */}
        <div className="bg-pop rounded-lg p-1.5">
          {/* Step label strip */}
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
                onPickFolder={handlePickFolder}
                onNext={(data) => {
                  setRootDir(data.rootDir);
                  setStep(2);
                }}
              />
            )}
            {step === 2 && (
              <AccountStep form={accountForm} error={setupError} onNext={handleFinish} />
            )}
            {step === 3 && <SettingUpStep />}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Step sub-components ───────────────────────────────────────────────────────

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
  onPickFolder,
  onNext,
}: {
  form: ReturnType<typeof useForm<StorageForm>>;
  rootDir: string;
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

      <Button type="submit" className="w-full">
        Continue
        <ChevronRight />
      </Button>
    </form>
  );
}

function AccountStep({
  form,
  error,
  onNext,
}: {
  form: ReturnType<typeof useForm<AccountForm>>;
  error: string | null;
  onNext: (data: AccountForm) => void;
}) {
  return (
    <form onSubmit={form.handleSubmit(onNext)} className="space-y-5">
      <div className="space-y-1.5">
        <h2 className="font-display text-2xl uppercase">Create account</h2>
        <p className="text-sm text-muted-foreground">
          Create your local account to access ARX Drive.
        </p>
      </div>

      <div className="space-y-3">
        <div className="space-y-1.5">
          <Label htmlFor="s-email" className="text-xs uppercase">
            Email
          </Label>
          <Input
            id="s-email"
            type="email"
            autoComplete="email"
            placeholder="you@example.com"
            {...form.register("email")}
          />
          {form.formState.errors.email && (
            <p className="text-xs text-destructive">
              {form.formState.errors.email.message}
            </p>
          )}
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="s-password" className="text-xs uppercase">
            Password
          </Label>
          <Input
            id="s-password"
            type="password"
            autoComplete="new-password"
            placeholder="••••••••"
            {...form.register("password")}
          />
          {form.formState.errors.password && (
            <p className="text-xs text-destructive">
              {form.formState.errors.password.message}
            </p>
          )}
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="s-confirm" className="text-xs uppercase">
            Confirm password
          </Label>
          <Input
            id="s-confirm"
            type="password"
            autoComplete="new-password"
            placeholder="••••••••"
            {...form.register("confirm")}
          />
          {form.formState.errors.confirm && (
            <p className="text-xs text-destructive">
              {form.formState.errors.confirm.message}
            </p>
          )}
        </div>
      </div>

      {error && (
        <p className="text-xs text-destructive bg-destructive/10 border border-destructive/30 rounded px-3 py-2">
          {error}
        </p>
      )}

      <Button type="submit" className="w-full">
        Create account
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
          Starting server and creating your account
        </p>
      </div>
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
  // Don't throw — server might be starting slowly; proceed anyway
}

// Unused but satisfies "done" icon usage for future
export { Check };
