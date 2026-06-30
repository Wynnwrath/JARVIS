/**
 * Thin wrapper around the Tauri CLI that injects environment-specific config
 * overrides. `bun run tauri dev` is extended with `src-tauri/tauri.dev.conf.json`
 * so development-only CSP/HMR settings stay out of release builds; release
 * builds use the production-hardened CSP in `src-tauri/tauri.conf.json`.
 */
const { spawn } = require("child_process");

const [_node, _script, subcommand, ...restArgs] = process.argv;

const prependArgs = [];
if (subcommand === "dev") {
  prependArgs.push("--config", "src-tauri/tauri.dev.conf.json");
}
// `tauri build` and `tauri bundle` intentionally use the base production config.

const args = subcommand ? [subcommand, ...prependArgs, ...restArgs] : [...prependArgs, ...restArgs];
const child = spawn("tauri", args, { stdio: "inherit", shell: true });
child.on("exit", (code) => {
  process.exit(code ?? 0);
});
