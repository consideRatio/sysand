# `sysand index`

Create and modify a local static sysand index tree.

```sh
sysand index <COMMAND>
```

The `sysand index` commands operate on the current directory. They write the
file layout described by the [Sysand index protocol](../index-protocol.md).

## Commands

```sh
sysand index init
sysand index add <KPAR_PATH> [--iri <IRI>]
sysand index yank <IRI> <VERSION>
sysand index remove <IRI> [--version <VERSION>]
```

- `init`: create an empty `index.json` in the current directory.
- `add`: add a local `.kpar` release. Without `--iri`, the command infers a
  `pkg:sysand/<publisher>/<name>` IRI from the KPAR project information.
- `yank`: mark one version as `yanked` in `versions.json`. Release files remain
  available.
- `remove --version`: mark one version as `removed` in `versions.json` and
  delete its release files.
- `remove`: mark the project as `removed` in `index.json`, mark all versions as
  `removed`, and delete their release files.

## Examples

```sh
sysand index init
sysand index add output/my-project-1.0.0.kpar
sysand index add vendor-project.kpar --iri urn:kpar:vendor-project
sysand index yank pkg:sysand/example/my-project 1.0.0
sysand index remove pkg:sysand/example/my-project --version 1.0.0
sysand index remove pkg:sysand/example/my-project
```
