# Build

The `[build]` section configures the behavior of the `sysand build` command.

## README bundling

By default, `sysand build` looks for a `README.md` file at the project root
and includes it in the `.kpar` archive. This allows package indexes to display
README content on package pages.

If no `README.md` file exists, the build proceeds normally without including one.

### Configuring the README source file

To use a different file as the README source:

```toml
[build]
readme = "PUBLIC_README.md"
```

The file will be stored as `README.md` inside the `.kpar` archive regardless of
the source filename.

### Disabling README bundling

To explicitly disable README bundling:

```toml
[build]
readme = ""
```
