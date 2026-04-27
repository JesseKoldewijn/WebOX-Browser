const target = Bun.argv[2] ?? "help";

const commands: Record<string, string> = {
  browser: "cargo run -p webox-browser-app",
  docs: "bun --cwd apps/docs run dev",
};

if (!(target in commands)) {
  console.log("Usage: bun run dev -- <browser|docs>");
  process.exit(0);
}

const child = Bun.spawn(commands[target].split(" "), {
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});

await child.exited;
