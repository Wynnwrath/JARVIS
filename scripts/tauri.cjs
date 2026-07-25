/**
 * Thin wrapper around the Tauri CLI that injects environment-specific config
 * overrides. `bun run tauri dev` is extended with `src-tauri/tauri.dev.conf.json`
 * so development-only CSP/HMR settings stay out of release builds; release
 * builds use the production-hardened CSP in `src-tauri/tauri.conf.json`.
 */
const { spawn, spawnSync } = require("child_process");

const [_node, _script, subcommand, ...restArgs] = process.argv;

const prependArgs = [];
if (subcommand === "dev") {
  prependArgs.push("--config", "src-tauri/tauri.dev.conf.json");
}
// `tauri build` and `tauri bundle` intentionally use the base production config.

const args = subcommand ? [subcommand, ...prependArgs, ...restArgs] : [...prependArgs, ...restArgs];
const child = spawn("tauri", args, { stdio: "inherit", shell: true });

let killed = false;
function killTree() {
  if (killed) return;
  killed = true;
  if (process.platform === "win32" && child.pid) {
    spawnSync("taskkill", ["/F", "/T", "/PID", String(child.pid)], { stdio: "ignore" });
  } else {
    try { child.kill("SIGKILL"); } catch (_) {}
  }
}

process.on("SIGINT", () => { killTree(); process.exit(130); });
process.on("SIGTERM", () => { killTree(); process.exit(143); });
process.on("SIGHUP", () => { killTree(); process.exit(129); });

child.on("exit", (code) => {
  killTree();
  process.exit(code ?? 0);
});
