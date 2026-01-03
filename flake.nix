{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v-utils.url = "github:valeratrades/.github";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v-utils, ... }:
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
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v-utils.files.preCommit { inherit pkgs; });
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          github = v-utils.github {
            inherit pkgs pname;
            lastSupportedVersion = "nightly-2025-10-12";
            langs = [ "rs" ];
            jobs.default = true;
          };
          rs = v-utils.rs { inherit pkgs; };
          readme = v-utils.readme-fw { inherit pkgs pname; defaults = true; lastSupportedVersion = "nightly-1.92"; rootDir = ./.; badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ]; };
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
              github.shellHook +
              rs.shellHook +
              readme.shellHook +
              ''
                cp -f ${(v-utils.files.treefmt) {inherit pkgs;}} ./.treefmt.toml
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
            ] ++ pre-commit-check.enabledPackages ++ github.enabledPackages;
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
                After = [ "network.target" "sops-nix.service" ];
                Requires = [ "sops-nix.service" ];
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
