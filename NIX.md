# Nix

This repository provides a Nix flake for developing, building, running, and installing Bottles Next.

The flake currently supports `x86_64-linux`.

## Development shell

To enter the development shell:

```bash
nix develop
```

The development shell includes:

- Rust and Cargo
- rust-analyzer
- rustfmt and Clippy
- protobuf and gRPC tools
- pkg-config
- CMake and Make
- OpenSSL
- the MinGW cross compiler
- the `x86_64-pc-windows-gnu` Rust target

The shell can build both the native Linux components and the Windows Wine bridge.

## Building

To build the complete project as a Nix package:

```bash
nix build
```

The default package contains:

```text
bin/bottles-cli
bin/bottles-server
libexec/bottles-next/bottles-winebridge.exe
```

The build output is available through the `result` symlink:

```bash
ls -R result
```

The default package can also be built explicitly:

```bash
nix build .#bottles-next
```

To show the full build log:

```bash
nix build --print-build-logs
```

## Running

To run the default application exposed by the flake:

```bash
nix run
```

The default application is `bottles-cli`.

It can also be run explicitly:

```bash
nix run .#bottles-cli
```

To run the server:

```bash
nix run .#bottles-server
```

Arguments can be passed after `--`:

```bash
nix run .#bottles-cli -- --help
```

```bash
nix run .#bottles-server -- --help
```

## Checking the flake

To build and validate all checks exposed by the flake:

```bash
nix flake check
```

To show full build logs:

```bash
nix flake check --print-build-logs
```

The checks verify that:

- the complete package builds;
- `bottles-cli` is installed;
- `bottles-server` is installed;
- `bottles-winebridge.exe` is installed;
- the installed files are in the expected locations.

## Formatting

To format the Nix files:

```bash
nix fmt
```

## Building from GitHub with Nix

If your Nix version supports automatic flake submodule fetching through:

```nix
inputs.self.submodules = true;
```

you can build directly from GitHub:

```bash
nix build github:bottlesdevs/bottles-next
```

To enter the development shell directly from GitHub:

```bash
nix develop github:bottlesdevs/bottles-next
```

To run the default application directly from GitHub:

```bash
nix run github:bottlesdevs/bottles-next
```

To run the server directly from GitHub:

```bash
nix run github:bottlesdevs/bottles-next#bottles-server
```

For older Nix versions, explicitly enable Git submodules:

```bash
nix build 'github:bottlesdevs/bottles-next?submodules=1'
```

The same applies to `nix develop` and `nix run`:

```bash
nix develop 'github:bottlesdevs/bottles-next?submodules=1'
```

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1'
```

To run the server with submodules explicitly enabled:

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1#bottles-server'
```

## Using the flake in NixOS or Home Manager

Add the repository as a flake input:

```nix
{
  inputs = {
    bottles-next.url = "github:bottlesdevs/bottles-next";
  };
}
```

For older Nix versions, explicitly enable Git submodules:

```nix
{
  inputs = {
    bottles-next.url = "github:bottlesdevs/bottles-next?submodules=1";
  };
}
```

It is recommended to make Bottles Next use the same Nixpkgs input as the consuming configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    bottles-next = {
      url = "github:bottlesdevs/bottles-next";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

## Home Manager

Install the package directly from the flake:

```nix
{ inputs, pkgs, ... }:

{
  home.packages = [
    inputs.bottles-next.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
```

On an explicitly configured `x86_64-linux` system:

```nix
{ inputs, ... }:

{
  home.packages = [
    inputs.bottles-next.packages.x86_64-linux.default
  ];
}
```

## NixOS

Install the package system-wide:

```nix
{ inputs, pkgs, ... }:

{
  environment.systemPackages = [
    inputs.bottles-next.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
```

On an explicitly configured `x86_64-linux` system:

```nix
{ inputs, ... }:

{
  environment.systemPackages = [
    inputs.bottles-next.packages.x86_64-linux.default
  ];
}
```

## Using the overlay

The flake exposes a default Nixpkgs overlay.

Add the overlay:

```nix
{ inputs, ... }:

{
  nixpkgs.overlays = [
    inputs.bottles-next.overlays.default
  ];
}
```

Bottles Next can then be installed as a regular package from `pkgs`:

```nix
{ pkgs, ... }:

{
  environment.systemPackages = [
    pkgs.bottles-next
  ];
}
```

The same works with Home Manager:

```nix
{ pkgs, ... }:

{
  home.packages = [
    pkgs.bottles-next
  ];
}
```

## Complete NixOS example

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    bottles-next = {
      url = "github:bottlesdevs/bottles-next";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      bottles-next,
      ...
    }:
    {
      nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";

        modules = [
          {
            nixpkgs.overlays = [
              bottles-next.overlays.default
            ];

            environment.systemPackages = [
              bottles-next.packages.x86_64-linux.default
            ];
          }
        ];
      };
    };
}
```

When using the overlay, the package can instead be referenced through `pkgs.bottles-next` inside a normal NixOS module.

## Updating dependencies

To update all flake inputs:

```bash
nix flake update
```

To update only Nixpkgs:

```bash
nix flake update nixpkgs
```

To update only the Rust overlay:

```bash
nix flake update rust-overlay
```

After updating inputs, verify that the project still builds:

```bash
nix flake check --print-build-logs
```

```bash
nix build --print-build-logs
```

Changes to `flake.lock` should be reviewed and committed together with the project.

## Dirty Git tree warnings

Nix may show a warning such as:

```text
warning: Git tree is dirty
```

This means that the repository contains uncommitted changes.

It does not normally prevent the flake from building.

View the changes with:

```bash
git status
```

Commit or discard the changes before publishing a release to ensure the build refers to a clean repository state.
