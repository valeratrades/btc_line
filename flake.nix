{
  inputs = {
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
  };

  outputs = { self, v_flakes }:
    let
      inherit (v_flakes) flake-utils pre-commit-hooks;
      manifest = (v_flakes.nixpkgs.lib.importTOML ./Cargo.toml).package;
      pname = manifest.name;
    in
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import v_flakes.default_nixpkgs {
            inherit system;
          };
          rust = v_flakes.rs.default_nightly system;
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          rs = v_flakes.rs { inherit pkgs rust; };
          github = v_flakes.github {
            inherit pkgs pname rs;
            enable = true;
            lastSupportedVersion = "nightly-${v_flakes.rs.nightly_version}";
            jobs.default = true;
            publishCachix = "valeratrades";
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
                RUSTC_WRAPPER = ""; # .cargo/config.toml sets sccache, absent in the sandbox

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
