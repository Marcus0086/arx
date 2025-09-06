{
  description = "ARX dev env (pinned Rust)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05"; # or a specific rev
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [ "aarch64-darwin" "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rust = pkgs.rust-bin.stable.latest.complete;

        native = with pkgs; [
          pkg-config
          zstd
          openssl
        ];
        dev = with pkgs; [
          rust
          rust-analyzer
          sccache
          just
          git
        ];
      in {
        # nix develop -> your dev shell
        devShells.default = pkgs.mkShell {
          packages = dev ++ native;

          # faster rebuilds
          RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

          shellHook = ''
            export CARGO_HOME="$PWD/.cargo-home"
            export RUSTUP_HOME="$PWD/.rustup-home"
            mkdir -p .cache/sccache
            export SCCACHE_DIR="$PWD/.cache/sccache"
            export SCCACHE_CACHE_SIZE="20G"
            echo "🦀 dev shell active (Rust pinned by flake)"
          '';
        };

        # nix build .  -> builds your crate reproducibly
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "arx";
          version = "0.0.1";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock; # commit your lockfile!
          nativeBuildInputs = native;
          # Example: enable a feature if needed
          # buildFeatures = [ "zstd" ];
        };

        # nix run .    -> runs the built binary
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };
      });
}
