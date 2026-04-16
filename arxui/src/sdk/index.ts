import { defaultTokenStore, type TokenStore } from "./token-store";
import { buildTransport } from "./transport";
import { AuthService } from "./auth-service";
import { VaultService } from "./vault-service";
import { FileService } from "./file-service";

export type { TokenStore } from "./token-store";
export type { LoginResult, WhoamiResult } from "./auth-service";
export type { Vault, CreateVaultOpts, VerifyResult } from "./vault-service";
export type { FileEntry, ProgressItem, DiffEntry, UploadFile } from "./file-service";

export interface ArxClientOpts {
  /** Base URL of the arx-grpc server (must have tonic-web enabled). */
  baseUrl: string;
  /** Pluggable token storage — defaults to memory (access) + localStorage (refresh). */
  tokenStore?: TokenStore;
  /** Called when a token refresh fails (e.g. expired refresh token). Redirect to login. */
  onAuthExpired?: () => void;
}

export class ArxClient {
  readonly auth: AuthService;
  readonly vaults: VaultService;
  readonly files: FileService;

  private constructor(auth: AuthService, vaults: VaultService, files: FileService) {
    this.auth = auth;
    this.vaults = vaults;
    this.files = files;
  }

  static create(opts: ArxClientOpts): ArxClient {
    const store = opts.tokenStore ?? defaultTokenStore;
    const onExpired = opts.onAuthExpired ?? (() => {
      if (typeof window !== "undefined") window.location.href = "/login";
    });

    const transport = buildTransport(opts.baseUrl, store, onExpired);

    return new ArxClient(
      new AuthService(transport, store),
      new VaultService(transport),
      new FileService(transport),
    );
  }
}
