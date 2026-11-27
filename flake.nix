{
  description = "reflectapi development shell: one toolchain for local work and CI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f system (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );

      # Codegen pipes generated code through these formatters, so their versions
      # are part of its output: the snapshot tests and the Python drift check
      # pass only with these exact releases. nixpkgs carries other versions, so
      # the upstream release binaries are pinned here.
      # When bumping biome, also update reflectapi/src/codegen/typescript.rs.
      biome = {
        version = "1.8.3";
        assets = {
          x86_64-linux = {
            name = "biome-linux-x64-musl";
            hash = "sha256-89sW03bXfNwo+AMdLi25XzYgxvHkY+VVu188RBtrluU=";
          };
          aarch64-linux = {
            name = "biome-linux-arm64-musl";
            hash = "sha256-T58qYyVbM1qAoZe4CgstXQAIbCkgXf57m2E4wBB3eYc=";
          };
          x86_64-darwin = {
            name = "biome-darwin-x64";
            hash = "sha256-LMESF4Nj+g4R2PJuDYBZjv9s5IGh7iJLioriuKUYUSQ=";
          };
          aarch64-darwin = {
            name = "biome-darwin-arm64";
            hash = "sha256-nXB8KO6OExWNK5iQ3Z+vBkpOptq94gCaaoYHSArowko=";
          };
        };
      };

      ruff = {
        version = "0.15.8";
        assets = {
          x86_64-linux = {
            triple = "x86_64-unknown-linux-musl";
            hash = "sha256-1UG+rpnVUO1KuzodAmuQeIbHzfRKUzskYkhx49jIEzA=";
          };
          aarch64-linux = {
            triple = "aarch64-unknown-linux-musl";
            hash = "sha256-FeamwhaWu+WcVtDxxDdFK5YLzf6B7MO8GfqJ5qfXDrY=";
          };
          x86_64-darwin = {
            triple = "x86_64-apple-darwin";
            hash = "sha256-FT0YAQaN9gYpDoMgWM4uVgFYSsMCeIoFXQOQrfbHcs4=";
          };
          aarch64-darwin = {
            triple = "aarch64-apple-darwin";
            hash = "sha256-lPwGH5KMjysExLOpiq0rGwTzi0yAiDm8WzOi8KY6R6M=";
          };
        };
      };
    in
    {
      devShells = forAllSystems (
        system: pkgs:
        let
          # The channel and components come from rust-toolchain.toml, so rustup
          # users and this shell build with the same compiler.
          toolchain = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml)).toolchain;
          rust = pkgs.rust-bin.stable.${toolchain.channel}.default.override {
            extensions = toolchain.components ++ [
              "rust-src"
              "rust-analyzer"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          };

          biomeBin = pkgs.stdenvNoCC.mkDerivation {
            pname = "biome";
            inherit (biome) version;
            src = pkgs.fetchurl {
              url = "https://github.com/biomejs/biome/releases/download/cli%2Fv${biome.version}/${biome.assets.${system}.name}";
              inherit (biome.assets.${system}) hash;
            };
            dontUnpack = true;
            installPhase = "install -Dm755 $src $out/bin/biome";
          };

          ruffBin = pkgs.stdenvNoCC.mkDerivation {
            pname = "ruff";
            inherit (ruff) version;
            src = pkgs.fetchurl {
              url = "https://github.com/astral-sh/ruff/releases/download/${ruff.version}/ruff-${ruff.assets.${system}.triple}.tar.gz";
              inherit (ruff.assets.${system}) hash;
            };
            installPhase = "install -Dm755 ruff $out/bin/ruff";
          };
        in
        {
          # mkShell brings a C compiler and make; reqwest's native-tls needs
          # openssl through pkg-config.
          default = pkgs.mkShell {
            packages = [
              rust
              biomeBin
              ruffBin
              pkgs.pkg-config
              pkgs.openssl
              pkgs.nodejs_24
              pkgs.pnpm_10
              pkgs.uv
              pkgs.python3
              pkgs.mdbook
            ];

            # The Python jobs test against uv-managed interpreters (3.12 and
            # 3.14), not the shell's python3, which only runs repo scripts.
            # On NixOS these need programs.nix-ld.
            UV_PYTHON_PREFERENCE = "only-managed";
          };
        }
      );
    };
}
