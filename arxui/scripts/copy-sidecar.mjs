import { execSync } from "child_process";
import { cpSync, mkdirSync, existsSync } from "fs";
import { join } from "path";

const repoRoot = new URL("../../..", import.meta.url).pathname;
const triple = execSync("rustc -Vv")
  .toString()
  .match(/host:\s+(\S+)/)?.[1];

if (!triple) {
  console.error("Could not detect target triple. Is rustc installed?");
  process.exit(1);
}

const src = join(repoRoot, "arx", "target", "release", "arx-grpc");
if (!existsSync(src)) {
  console.error(`Binary not found at ${src}`);
  console.error("Run: cargo build -p arx-grpc --release");
  process.exit(1);
}

const binDir = join(import.meta.dirname, "..", "src-tauri", "binaries");
mkdirSync(binDir, { recursive: true });

const dest = join(binDir, `arx-grpc-${triple}`);
cpSync(src, dest);
console.log(`Copied arx-grpc → src-tauri/binaries/arx-grpc-${triple}`);
