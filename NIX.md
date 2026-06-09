# Nix

This repository provides a Nix flake for developing, building, running, and installing Bottles Next.

The flake currently supports `x86_64-linux`.

This repository uses Git submodules. When fetching the project directly from GitHub or adding it as an input to another flake, enable submodule fetching with `?submodules=1`.

## Local development

When the repository has already been cloned together with its Git submodules, Nix can use the local source directly.

Clone the repository with submodules:

```bash
git clone --recurse-submodules https://github.com/bottlesdevs/bottles-next.git
cd bottles-next
```

If the repository was cloned without submodules, initialize them afterward:

```bash
git submodule update --init --recursive
```

## Development shell

To enter the development shell:

```bash
nix develop
```

The development shell includes:

* Rust and Cargo
* rust-analyzer
* rustfmt and Clippy
* protobuf and gRPC tools
* pkg-config
* CMake and Make
* OpenSSL
* the MinGW cross compiler
* the `x86_64-pc-windows-gnu` Rust target

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

* the complete package builds;
* `bottles-cli` is installed;
* `bottles-server` is installed;
* `bottles-winebridge.exe` is installed;
* the installed files are in the expected locations.

## Formatting

To format the Nix files:

```bash
nix fmt
```

## Building from GitHub with Nix

This repository uses Git submodules, so submodule fetching must be enabled in the GitHub flake URL.

To build directly from GitHub:

```bash
nix build 'github:bottlesdevs/bottles-next?submodules=1'
```

To enter the development shell directly from GitHub:

```bash
nix develop 'github:bottlesdevs/bottles-next?submodules=1'
```

To run the default application directly from GitHub:

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1'
```

To run the CLI explicitly:

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1#bottles-cli'
```

To run the server:

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1#bottles-server'
```

Arguments can be passed after `--`:

```bash
nix run 'github:bottlesdevs/bottles-next?submodules=1#bottles-cli' -- --help
```

## Using the flake in NixOS or Home Manager

Add Bottles Next as an input with Git submodules enabled:

```nix
{
  inputs = {
    bottles-next.url =
      "github:bottlesdevs/bottles-next?submodules=1";
  };
}
```

It is recommended to make Bottles Next use the same Nixpkgs input as the consuming configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    bottles-next = {
      url = "github:bottlesdevs/bottles-next?submodules=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

If using a fork, replace the repository owner:

```nix
{
  inputs = {
    bottles-next = {
      url = "github:hotplugindev/bottles-next?submodules=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

After adding or changing the input, update the lock file:

```bash
nix flake update bottles-next
```

On older Nix versions, use:

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

Rebuild the system after adding the package:

```bash
sudo nixos-rebuild switch --flake .#your-hostname
```

For example:

```bash
sudo nixos-rebuild switch --flake ~/nixos#pc
```

## Using the overlay

The flake exposes a default Nixpkgs overlay.

Add the overlay to the consuming configuration:

```nix
{ inputs, ... }:

{
  nixpkgs.overlays = [
    inputs.bottles-next.overlays.default
  ];
}
```

Bottles Next can then be installed as a normal package from `pkgs`:

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

Normally, use either the package output directly or the overlay. Using both is unnecessary.

## Complete NixOS example

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    bottles-next = {
      url = "github:bottlesdevs/bottles-next?submodules=1";
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
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    bottles-next = {
      url = "github:bottlesdevs/bottles-next?submodules=1";
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
              pkgs.bottles-next
            ];
          }
        ];
      };
    };
}
```

When writing the module inline as shown above, `pkgs` must be made available through module arguments:

```nix
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
}
```

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

Commit or discard the changes before publishing a release so the build refers to a clean repository state.

## Troubleshooting submodules

If a local build fails because files from a submodule are missing, initialize the submodules:

```bash
git submodule update --init --recursive
```

If a build from GitHub fails because submodules are missing, confirm that the flake URL contains:

```text
?submodules=1
```

For example:

```bash
nix build 'github:bottlesdevs/bottles-next?submodules=1'
```

Do not add this to the Bottles Next flake itself:

```nix
inputs.self.submodules = true;
```

Some Nix versions reject that attribute when the flake is fetched through the `github:` scheme. Using `?submodules=1` in the consuming flake URL is more broadly compatible.
