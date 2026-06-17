{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/549bd84d6279f9852cae6225e372cc67fb91a4c1";
    rust-overlay.url = "github:oxalica/rust-overlay/adf987c76af8d17b8256d23631bcf203f81e1a63";
    flake-utils.url = "github:numtide/flake-utils/11707dc2f618dd54ca8739b309ec4fc024de578b";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix/3cfd774b0a530725a077e17354fbdb87ea1c4aad";
    v_flakes.url = "github:valeratrades/v_flakes/d4737aa179386874334cb1b2b21f174d11957fc4";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v_flakes }:
    let
      manifest = (nixpkgs.lib.importTOML ./Cargo.toml).package;
      pname = manifest.name;
    in
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = builtins.trace "flake.nix sourced" [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
            extensions = [ "rust-src" "rust-analyzer" "rust-docs" "rustc-codegen-cranelift-preview" ];
          });
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          rs = v_flakes.rs { inherit pkgs rust; };
          github = v_flakes.github {
            inherit pkgs pname rs;
            enable = true;
            lastSupportedVersion = "nightly-2025-10-12";
            jobs.default = true;
          };
          readme = v_flakes.readme-fw { inherit pkgs pname; defaults = true; lastSupportedVersion = "nightly-1.92"; rootDir = ./.; badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ]; };
          combined = v_flakes.utils.combine { inherit rust; modules = [ rs github readme ]; };
        in
        {
          packages =
            let
              rustc = rust;
              cargo = rust;
              rustPlatform = pkgs.makeRustPlatform {
                inherit rustc cargo stdenv;
              };
            in
            {
              default = rustPlatform.buildRustPackage {
                inherit pname;
                version = manifest.version;

                buildInputs = with pkgs; [
                  openssl.dev
                ];
                nativeBuildInputs = with pkgs; [ pkg-config ];

                cargoLock.lockFile = ./Cargo.lock;
                src = pkgs.lib.cleanSource ./.;
              };
            };

          devShells.default = with pkgs; mkShell {
            inherit stdenv;
            shellHook =
              pre-commit-check.shellHook +
              combined.shellHook +
              ''
                cp -f ${(v_flakes.files.treefmt) {inherit pkgs;}} ./.treefmt.toml
              '';
            env = {
              RUST_BACKTRACE = 1;
              RUST_LIB_BACKTRACE = 0;
            };

            packages = [
              mold
              openssl
              pkg-config
              rust
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;
          };
        }
      )
    // {
      homeManagerModules."${pname}" = { config, lib, pkgs, ... }:
        let
          inherit (lib) mkEnableOption mkOption mkIf;
          inherit (lib.types) package path;
          cfg = config."${pname}";
        in
        {
          options."${pname}" = {
            enable = mkEnableOption "";

            package = mkOption {
              type = package;
              default = self.packages.${pkgs.system}.default;
              description = "The package to use.";
            };

            alpacaKey = mkOption {
              type = path;
              description = "Path to file containing Alpaca API key";
            };

            alpacaSecret = mkOption {
              type = path;
              description = "Path to file containing Alpaca API secret";
            };
          };

          config = mkIf cfg.enable {
            systemd.user.services.btc_line = {
              Unit = {
                Description = "btc_line";
                After = [ "network-online.target" "sops-nix.service" ];
                Wants = [ "network-online.target" "sops-nix.service" ];
              };

              Install = {
                WantedBy = [ "default.target" ];
              };

              Service = {
                Type = "simple";
                Environment = [
                  "PATH=/run/current-system/sw/bin:/etc/profiles/per-user/%u/bin"
                  "HOME=%h"
                  "XDG_STATE_HOME=%h/.local/state"
                  "ALPACA_API_KEY=placeholder"
                  "ALPACA_API_SECRET=placeholder"
                ];
                LoadCredential = [
                  "alpaca_key:${cfg.alpacaKey}"
                  "alpaca_secret:${cfg.alpacaSecret}"
                ];
                ExecStartPre = "${pkgs.bash}/bin/bash -c 'while [ ! -f ${cfg.alpacaKey} ] || [ ! -f ${cfg.alpacaSecret} ]; do ${pkgs.coreutils}/bin/sleep 0.1; done'";
                ExecStart = "/bin/sh -c '${cfg.package}/bin/${pname} --spy-alpaca-key \"$(cat %d/alpaca_key)\" --spy-alpaca-secret \"$(cat %d/alpaca_secret)\"'";
                Restart = "on-failure";
                RestartSec = 5;
              };
            };

            home.packages = [ cfg.package ];
          };
        };
    };
}
