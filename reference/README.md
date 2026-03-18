# Sysand: a package manager for SysML v2 and KerML

This repository contains Sysand, a [package
manager](https://en.wikipedia.org/wiki/Package_manager) for SysML v2 and KerML
similar to package managers for programming languages such as Pip for Python,
NPM for JavaScript, Maven for Java, and NuGet for .NET. Sysand is based on a
concept of a model interchange project, a slight generalization of a project
interchange file (`*.kpar`), defined in [KerML clause
10.3](https://www.omg.org/spec/KerML/1.0/PDF#page=432).

## Running tests

```sh
cargo test -p sysand-core -F filesystem,networking,js,python,alltests,kpar-bzip2,kpar-zstd,kpar-xz,kpar-ppmd
cargo test -p sysand -F alltests,kpar-bzip2,kpar-zstd,kpar-xz,kpar-ppmd
```
