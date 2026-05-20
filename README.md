# Bottles Next

Central repository for the Bottles Next project.

## Usage

After cloning the repository, download the submodules:

```bash
git submodule update --init --recursive
```

Or clone the repository with submodules directly:

```bash
git clone --recurse-submodules https://github.com/bottlesdevs/bottles-next.git
```

This project contains the following submodules:

- [next-cli](https://github.com/bottlesdevs/next-cli)
- [next-core](https://github.com/bottlesdevs/next-core)
- [next-server](https://github.com/bottlesdevs/next-server)
- [next-winebridge](https://github.com/bottlesdevs/next-winebridge)
- [next-docs](https://github.com/bottlesdevs/next-docs)

## Build

To build the project, you can use the `just` command:

```bash
just build
```

## Nix

This repository provides a Nix flake.

To enter the development shell:

```bash
nix develop
```

To build the project as a Nix package:

```bash
nix build
```

The build output will be available through the `result` symlink:

```bash
ls -R result
```

To run the default app exposed by the flake:

```bash
nix run
```

### Building from GitHub with Nix

If your Nix version supports automatic flake submodule fetching through `inputs.self.submodules = true`, you can build directly with:

```bash
nix build github:bottlesdevs/bottles-next
```

For older Nix versions, explicitly enable submodules:

```bash
nix build 'github:bottlesdevs/bottles-next?submodules=1'
```

The same applies to `nix develop` and `nix run`:

```bash
nix develop 'github:bottlesdevs/bottles-next?submodules=1'
nix run 'github:bottlesdevs/bottles-next?submodules=1'
```

### Using the flake in NixOS or Home Manager

Add the repository as an input:

```nix
{
  inputs = {
    bottles-next.url = "github:bottlesdevs/bottles-next";
  };
}
```

For older Nix versions, use:

```nix
{
  inputs = {
    bottles-next.url = "github:bottlesdevs/bottles-next?submodules=1";
  };
}
```

Then install the package from the flake:

```nix
home.packages = [
  inputs.bottles-next.packages.x86_64-linux.default
];
```

or system-wide on NixOS:

```nix
environment.systemPackages = [
  inputs.bottles-next.packages.x86_64-linux.default
];
```

## License

GPL-3.0
