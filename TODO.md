- When the CLI runs, it becomes aware about the current working directory.
  However libraries don't have information like that out-of-the box. So, what do
  we in order to make up for that difference?
- sysand.toml config may be relevant, but its project specific, so what project
  are we working on if we don't quickly detect it from the CWD?
- Respecting .workspace.json and sysand.toml config, how to do it in cli/library/bindings?
