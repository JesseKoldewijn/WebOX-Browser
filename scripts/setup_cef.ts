import { mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";

const root = join(process.cwd(), "third_party", "cef", "linux-x64");
const directories = [
  root,
  join(root, "bin"),
  join(root, "resources"),
  join(root, "locales"),
];

for (const directory of directories) {
  if (!existsSync(directory)) {
    await mkdir(directory, { recursive: true });
  }
}

const notesPath = join(root, "README.md");
const notes = `# CEF Runtime Staging\n\nThis directory is reserved for the Linux x86_64 CEF distribution used by webox.\n\nExpected contents:\n- libcef shared library and related runtime files\n- locales/\n- resources/\n- bin/webox-cef-subprocess\n\nProvisioning is currently manual. Place the selected CEF distribution here and ensure the subprocess binary path matches crates/config/src/lib.rs.\n`;

await Bun.write(notesPath, notes);
console.log(`Prepared CEF runtime staging directories at ${root}`);
