const _ws = process.env.CLI_WORKSPACE || process.env.CLAUDE_CODE_WORKSPACE;
if (_ws && _ws !== process.cwd()) {
  try { process.chdir(_ws) } catch {}
}
