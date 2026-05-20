{
  description = "Bottles Next";

  inputs = {
    self.submodules = true;

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";

      overlays = [
        rust-overlay.overlays.default
      ];

      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = [
          "x86_64-pc-windows-gnu"
        ];
      };

      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };

      mingw = pkgs.pkgsCross.mingwW64;
      mingwCc = mingw.stdenv.cc;
      mingwPthreads = mingw.windows.pthreads;
    in
    {
      packages.${system} = rec {
        default = bottles-next;

        bottles-next = rustPlatform.buildRustPackage {
          pname = "bottles-next";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

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

            cargo build --frozen --release --package bottles-cli
            cargo build --frozen --release --package bottles-core
            cargo build --frozen --release --package bottles-server
            cargo build --frozen --release --package bottles-winebridge --target x86_64-pc-windows-gnu

            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall

            mkdir -p $out/bin
            mkdir -p $out/libexec/bottles-next

            cp target/release/bottles-cli $out/bin/
            cp target/release/bottles-server $out/bin/

            cp target/x86_64-pc-windows-gnu/release/bottles-winebridge.exe \
              $out/libexec/bottles-next/

            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Bottles Next";
            homepage = "https://github.com/bottlesdevs/bottles-next";
            license = licenses.gpl3Only;
            platforms = platforms.linux;
          };
        };
      };

      apps.${system} = {
        default = self.apps.${system}.bottles-cli;

        bottles-cli = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/bottles-cli";
        };

        bottles-server = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/bottles-server";
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain

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

        shellHook = ''
          export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
          export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_AR=x86_64-w64-mingw32-ar
          export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-Lnative=${mingwPthreads}/lib"

          export PROTOC="${pkgs.protobuf}/bin/protoc"

          echo "Entered Bottles Next development shell"
          echo "Rust target: x86_64-pc-windows-gnu"
          echo "Linker: x86_64-w64-mingw32-gcc"
          echo "MinGW pthreads: ${mingwPthreads}/lib"
        '';
      };
    };
}
