if (process.env.CLI_WORKSPACE && process.env.CLI_WORKSPACE !== process.cwd()) {
  try { process.chdir(process.env.CLI_WORKSPACE) } catch {}
}
