{
  description = "A simple rust flake using rust-overlay and craneLib";

  inputs = {
    main.url = "github:uttarayan21/templates";
    nixpkgs.follows = "main/nixpkgs";
    flake-utils.follows = "main/flake-utils";
    crane.follows = "main/crane";
    nix-github-actions.follows = "main/nix-github-actions";
    rust-overlay.follows = "main/rust-overlay";
    advisory-db.follows = "main/advisory-db";
  };

  outputs = {
    self,
    crane,
    flake-utils,
    nixpkgs,
    rust-overlay,
    advisory-db,
    nix-github-actions,
    ...
  }: let
    # Read outside eachSystem: the crate name is system-independent, and the
    # githubActions matrix below needs it to tell the canonical checks apart
    # from their unprefixed aliases.
    cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    name = cargoToml.package.name;
  in
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ] (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
          ];
        };
        inherit (pkgs) lib;

        stableToolchain = pkgs.rust-bin.stable.latest.default;
        stableToolchainWithLLvmTools = stableToolchain.override {
          extensions = ["rust-src" "llvm-tools"];
        };
        stableToolchainWithRustAnalyzer = stableToolchain.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain stableToolchain;
        craneLibLLvmTools = (crane.mkLib pkgs).overrideToolchain stableToolchainWithLLvmTools;

        src = let
          filterBySuffix = path: exts: lib.any (ext: lib.hasSuffix ext path) exts;
          sourceFilters = path: type: (craneLib.filterCargoSources path type) || filterBySuffix path [".c" ".h" ".hpp" ".cpp" ".cc"];
        in
          lib.cleanSourceWith {
            filter = sourceFilters;
            src = ./.;
          };
        commonArgs =
          {
            inherit src;
            pname = name;
            stdenv = p: p.clangStdenv;
            doCheck = false;
            # LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            # nativeBuildInputs = with pkgs; [
            #   cmake
            #   llvmPackages.libclang.lib
            # ];
            buildInputs = with pkgs;
              []
              ++ (lib.optionals pkgs.stdenv.isDarwin [
                libiconv
                apple-sdk_26
              ]);
          }
          // (lib.optionalAttrs pkgs.stdenv.isLinux {
            # BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.llvmPackages.libclang.lib}/lib/clang/18/include";
          });
        cargoArtifacts = craneLib.buildPackage commonArgs;
      in {
        checks = let
          # Keyed by what the check is rather than what the crate is called, so
          # that CI can reference a fixed attribute path without knowing the
          # package name.
          byKind =
            {
              clippy = craneLib.cargoClippy (commonArgs
                // {
                  inherit cargoArtifacts;
                  cargoClippyExtraArgs = "--all-targets -- --deny warnings";
                });
              docs = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});
              fmt = craneLib.cargoFmt {inherit src;};
              toml-fmt = craneLib.taploFmt {
                src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
              };
              # Audit dependencies
              audit = craneLib.cargoAudit {
                inherit src advisory-db;
              };

              # Audit licenses
              deny = craneLib.cargoDeny {
                inherit src;
              };
              nextest = craneLib.cargoNextest (commonArgs
                // {
                  inherit cargoArtifacts;
                  partitions = 1;
                  partitionType = "count";
                });
            }
            // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
              llvm-cov = craneLibLLvmTools.cargoLlvmCov (commonArgs // {inherit cargoArtifacts;});
            };
        in
          # Both spellings are exposed, pointing at the same derivations:
          # `nix flake check` and the CI matrix read the prefixed names, while
          # anything that cannot know the crate name uses the bare ones.
          byKind
          // lib.mapAttrs' (kind: check: lib.nameValuePair "${name}-${kind}" check) byKind;

        packages = let
          pkg = craneLib.buildPackage (commonArgs
            // {inherit cargoArtifacts;}
            // {
              postInstall = ''
                mkdir -p $out/bin
                mkdir -p $out/share/bash-completions
                mkdir -p $out/share/fish/vendor_completions.d
                mkdir -p $out/share/zsh/site-functions
                $out/bin/${name} completions bash > $out/share/bash-completions/${name}.bash
                $out/bin/${name} completions fish > $out/share/fish/vendor_completions.d/${name}.fish
                $out/bin/${name} completions zsh > $out/share/zsh/site-functions/_${name}
              '';
            });
        in {
          "${name}" = pkg;
          default = pkg;
        };

        devShells = {
          default = pkgs.mkShell.override {stdenv = pkgs.clangStdenv;} (commonArgs
            // {
              packages = with pkgs;
                [
                  stableToolchainWithRustAnalyzer
                  cargo-nextest
                  cargo-deny
                ]
                ++ (lib.optionals pkgs.stdenv.isDarwin [
                  apple-sdk_26
                ]);
            });
        };
      }
    )
    // {
      githubActions = nix-github-actions.lib.mkGithubMatrix {
        # Only the prefixed checks; the bare aliases are the same derivations
        # and would otherwise double every row of the matrix.
        checks =
          builtins.mapAttrs
          (_system: nixpkgs.lib.filterAttrs (attr: _: nixpkgs.lib.hasPrefix "${name}-" attr))
          (nixpkgs.lib.getAttrs ["x86_64-linux"] self.checks);
      };
    };
}
