{
  description = "Bottles Next";

  inputs = {
    self.submodules = true;

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      system = "x86_64-linux";
      windowsTarget = "x86_64-pc-windows-gnu";

      overlays = [
        rust-overlay.overlays.default
      ];

      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = [
          windowsTarget
        ];

        extensions = [
          "clippy"
          "rust-src"
          "rustfmt"
        ];
      };

      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };

      mingw = pkgs.pkgsCross.mingwW64;
      mingwCc = mingw.stdenv.cc;
      mingwPthreads = mingw.windows.pthreads;

      bottlesNext = rustPlatform.buildRustPackage {
        pname = "bottles-next";
        version = "0.1.0";

        src = pkgs.lib.cleanSourceWith {
          src = ./.;

          filter =
            path: type:
            let
              name = builtins.baseNameOf path;
            in
            !builtins.elem name [
              ".direnv"
              ".env"
              ".envrc"
              ".git"
              ".idea"
              ".vscode"
              "result"
              "target"
            ];
        };

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        strictDeps = true;
        doCheck = false;

        nativeBuildInputs = with pkgs; [
          pkg-config
          protobuf
          grpc-tools
          cmake
          gnumake
          perl
          mingwCc
        ];

        buildInputs = with pkgs; [
          openssl
        ];

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "x86_64-w64-mingw32-gcc";

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_AR = "x86_64-w64-mingw32-ar";

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-Lnative=${mingwPthreads}/lib";

        PROTOC = "${pkgs.protobuf}/bin/protoc";

        buildPhase = ''
          runHook preBuild

          cargo build \
            --frozen \
            --release \
            --package bottles-cli

          cargo build \
            --frozen \
            --release \
            --package bottles-core

          cargo build \
            --frozen \
            --release \
            --package bottles-server

          cargo build \
            --frozen \
            --release \
            --package bottles-winebridge \
            --target ${windowsTarget}

          runHook postBuild
        '';

        installPhase = ''
          runHook preInstall

          install -Dm755 \
            target/release/bottles-cli \
            "$out/bin/bottles-cli"

          install -Dm755 \
            target/release/bottles-server \
            "$out/bin/bottles-server"

          install -Dm755 \
            target/${windowsTarget}/release/bottles-winebridge.exe \
            "$out/libexec/bottles-next/bottles-winebridge.exe"

          runHook postInstall
        '';

        doInstallCheck = true;

        installCheckPhase = ''
          runHook preInstallCheck

          test -x "$out/bin/bottles-cli"
          test -x "$out/bin/bottles-server"

          test -s \
            "$out/libexec/bottles-next/bottles-winebridge.exe"

          runHook postInstallCheck
        '';

        passthru = {
          inherit windowsTarget;
        };

        meta = {
          description = "Next-generation Bottles components";
          homepage = "https://github.com/bottlesdevs/bottles-next";
          license = pkgs.lib.licenses.gpl3Only;
          platforms = [
            "x86_64-linux"
          ];
          mainProgram = "bottles-cli";
        };
      };
    in
    {
      packages.${system} = {
        default = bottlesNext;
        bottles-next = bottlesNext;
      };

      apps.${system} = {
        default = {
          type = "app";
          program = "${bottlesNext}/bin/bottles-cli";

          meta = {
            description = "Run the Bottles Next CLI";
          };
        };

        bottles-cli = {
          type = "app";
          program = "${bottlesNext}/bin/bottles-cli";

          meta = {
            description = "Run the Bottles Next CLI";
          };
        };

        bottles-server = {
          type = "app";
          program = "${bottlesNext}/bin/bottles-server";

          meta = {
            description = "Run the Bottles Next server";
          };
        };
      };

      checks.${system} = {
        package = bottlesNext;

        package-layout = pkgs.runCommand "bottles-next-package-layout" { } ''
          test -x "${bottlesNext}/bin/bottles-cli"
          test -x "${bottlesNext}/bin/bottles-server"

          test -s \
            "${bottlesNext}/libexec/bottles-next/bottles-winebridge.exe"

          touch "$out"
        '';
      };

      formatter.${system} = pkgs.nixfmt;

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain
          rust-analyzer

          just

          grpc-tools
          protobuf
          pkg-config

          mingwCc

          cmake
          gnumake
          perl
          openssl
        ];

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "x86_64-w64-mingw32-gcc";

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_AR = "x86_64-w64-mingw32-ar";

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-Lnative=${mingwPthreads}/lib";

        PROTOC = "${pkgs.protobuf}/bin/protoc";

        shellHook = ''
          echo
          echo "Bottles Next development environment"
          echo
          echo "Native target:      ${system}"
          echo "Wine bridge target: ${windowsTarget}"
          echo "Windows linker:     x86_64-w64-mingw32-gcc"
          echo "MinGW pthreads:      ${mingwPthreads}/lib"
          echo
        '';
      };

      overlays.default = final: _previous: {
        bottles-next = self.packages.${final.stdenv.hostPlatform.system}.default;
      };
    };
}
