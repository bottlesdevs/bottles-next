# Nix

This repository provides a Nix flake for developing, building, running, and installing Bottles Next.

The flake currently supports `x86_64-linux`.

Bottles Next uses Git submodules for its Cargo workspace crates. The flake enables automatic submodule handling through:

```nix
inputs.self.submodules = true;
```

A recent Nix version is therefore required. Nix 2.27 or newer is recommended.

## Local development

Clone the repository:

```bash
git clone https://github.com/bottlesdevs/bottles-next.git
cd bottles-next
```

Initialize all Git submodules:

```bash
git submodule update --init --recursive
```

Alternatively, clone the repository and its submodules in one command:

```bash
git clone --recurse-submodules \
  https://github.com/bottlesdevs/bottles-next.git

cd bottles-next
```

Verify the submodules:

```bash
git submodule status
```

A line beginning with `-` means that the corresponding submodule has not been initialized.

## Development shell

Enter the development shell:

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

The development shell supports building both the native Linux components and the Windows Wine bridge.

## Building

Build the complete project as a Nix package:

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

Build the package explicitly by name:

```bash
nix build .#bottles-next
```

Show the complete build log:

```bash
nix build --print-build-logs
```

## Running

Run the default application:

```bash
nix run
```

The default application is `bottles-cli`.

Run the CLI explicitly:

```bash
nix run .#bottles-cli
```

Run the server:

```bash
nix run .#bottles-server
```

Pass arguments after `--`:

```bash
nix run .#bottles-cli -- --help
```

```bash
nix run .#bottles-server -- --help
```

## Checking the flake

Build and validate all checks exposed by the flake:

```bash
nix flake check
```

Show complete build logs:

```bash
nix flake check --print-build-logs
```

The checks verify that:

- the complete package builds;
- `bottles-cli` is installed;
- `bottles-server` is installed;
- `bottles-winebridge.exe` is installed;
- the installed files use the expected paths.

## Formatting

Format the Nix files:

```bash
nix fmt
```

## Building directly from the remote repository

The repository uses Git submodules.

For reliable remote builds, use the generic Git fetcher and enable submodules explicitly:

```bash
nix build \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1'
```

Do not use the shorter `github:` reference for remote builds. Depending on the installed Nix version, the GitHub-specific fetcher may return the parent repository without the contents of its submodules.

### Development shell from the remote repository

```bash
nix develop \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1'
```

### Run the default application from the remote repository

```bash
nix run \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1'
```

### Run the CLI explicitly

```bash
nix run \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1#bottles-cli'
```

### Run the server

```bash
nix run \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1#bottles-server'
```

Arguments can be passed after `--`:

```bash
nix run \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1#bottles-cli' \
  -- --help
```

## Building a specific branch or revision

Build a specific branch:

```bash
nix build \
  'git+https://github.com/bottlesdevs/bottles-next.git?ref=main&submodules=1'
```

Build a fork:

```bash
nix build \
  'git+https://github.com/hotplugindev/bottles-next.git?ref=branch-name&submodules=1'
```

Build an exact commit:

```bash
nix build \
  'git+https://github.com/bottlesdevs/bottles-next.git?rev=COMMIT_HASH&submodules=1'
```

Using an exact commit gives the most predictable remote build.

## Using the flake in NixOS or Home Manager

Add Bottles Next as an explicit Git input with submodules enabled:

```nix
{
  inputs = {
    bottles-next = {
      type = "git";
      url = "https://github.com/bottlesdevs/bottles-next.git";
      submodules = true;
    };
  };
}
```

It is recommended to make Bottles Next use the same Nixpkgs input as the consuming configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    bottles-next = {
      type = "git";
      url = "https://github.com/bottlesdevs/bottles-next.git";
      submodules = true;

      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

When testing a fork or branch:

```nix
{
  inputs = {
    bottles-next = {
      type = "git";
      url = "https://github.com/hotplugindev/bottles-next.git";
      ref = "branch-name";
      submodules = true;

      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

After adding or changing the input, update the lock file:

```bash
nix flake update bottles-next
```

On older Nix versions:

```bash
nix flake lock --update-input bottles-next
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

Because Bottles Next currently supports only `x86_64-linux`, it can also be referenced explicitly:

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

It can also be referenced explicitly:

```nix
{ inputs, ... }:

{
  environment.systemPackages = [
    inputs.bottles-next.packages.x86_64-linux.default
  ];
}
```

Rebuild the system:

```bash
sudo nixos-rebuild switch --flake .#your-hostname
```

For example:

```bash
sudo nixos-rebuild switch --flake ~/nixos#pc
```

## Using the overlay

The flake exposes a default Nixpkgs overlay.

Add it to the consuming configuration:

```nix
{ inputs, ... }:

{
  nixpkgs.overlays = [
    inputs.bottles-next.overlays.default
  ];
}
```

Bottles Next can then be installed through `pkgs`:

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

Use either the package output directly or the overlay. Using both is unnecessary.

## Complete NixOS example

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    bottles-next = {
      type = "git";
      url = "https://github.com/bottlesdevs/bottles-next.git";
      submodules = true;

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
            environment.systemPackages = [
              bottles-next.packages.x86_64-linux.default
            ];
          }
        ];
      };
    };
}
```

## Complete NixOS example using the overlay

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    bottles-next = {
      type = "git";
      url = "https://github.com/bottlesdevs/bottles-next.git";
      submodules = true;

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
          (
            { pkgs, ... }:
            {
              nixpkgs.overlays = [
                bottles-next.overlays.default
              ];

              environment.systemPackages = [
                pkgs.bottles-next
              ];
            }
          )
        ];
      };
    };
}
```

## Updating dependencies

Update all flake inputs:

```bash
nix flake update
```

Update only Nixpkgs:

```bash
nix flake update nixpkgs
```

Update only the Rust overlay:

```bash
nix flake update rust-overlay
```

After updating inputs, verify the project:

```bash
nix flake check --print-build-logs
```

```bash
nix build --print-build-logs
```

Review and commit changes to `flake.lock` together with the corresponding project changes.

## Dirty Git tree warnings

Nix may show:

```text
warning: Git tree is dirty
```

This means that the repository contains uncommitted changes.

It does not normally prevent the flake from building.

Inspect the changes with:

```bash
git status
```

Commit or discard the changes before publishing a release so that the release refers to a clean repository state.

## Troubleshooting submodules

### Local build reports a missing workspace crate

For example:

```text
failed to read crates/download-manager/Cargo.toml
No such file or directory
```

Initialize all submodules:

```bash
git submodule update --init --recursive
```

Verify them:

```bash
git submodule status
```

Then retry:

```bash
nix build
```

### Remote build reports a missing workspace crate

Use the generic Git fetcher rather than the `github:` shorthand:

```bash
nix build \
  'git+https://github.com/bottlesdevs/bottles-next.git?submodules=1'
```

For a NixOS or Home Manager input, use:

```nix
bottles-next = {
  type = "git";
  url = "https://github.com/bottlesdevs/bottles-next.git";
  submodules = true;
};
```

### Nix rejects `inputs.self.submodules`

The flake requires a recent Nix version for automatic local submodule handling.

Check the installed version:

```bash
nix --version
```

Nix 2.27 or newer is recommended.

On NixOS, a recent Nix package can be selected with:

```nix
nix.package = pkgs.nixVersions.latest;
```

Rebuild the system and verify the version before using the flake.

### A submodule uses an SSH URL

Remote Nix builds should not depend on SSH authentication.

All public submodules should use HTTPS URLs in `.gitmodules`, for example:

```ini
[submodule "crates/download-manager"]
    path = crates/download-manager
    url = https://github.com/bottlesdevs/download-manager.git
```

After changing a submodule URL:

```bash
git submodule sync --recursive
git submodule update --init --recursive
```
