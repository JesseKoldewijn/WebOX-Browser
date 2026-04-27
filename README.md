# webox

webox is a monorepo for a high-memory browser initiative that targets Chromium-class compatibility with a higher per-tab memory ceiling.

## Workspace Layout

- `apps/browser` - standalone browser application entrypoint
- `apps/docs` - Astro docs and marketing website with SolidJS islands
- `crates/*` - Rust workspace crates for shell, engine, memory, UI, runtime, and shared config
- `packages/*` - shared JavaScript and TypeScript packages for brand assets and shared contracts
- `docs/` - project documentation and architecture notes
- `validation/` - workload fixtures and validation plans

## Development

- `cargo test` - run Rust unit tests and smoke tests
- `cargo run -p webox-browser-app` - run the browser shell prototype
- `cargo run -p webox-workload-harness -- supported` - run the supported-system workload harness
- `cargo run -p webox-workload-harness -- constrained` - run the constrained-system workload harness
- `bun install` - install Astro, SolidJS, and workspace packages
- `bun run docs:dev` - run the docs site locally
- `bun run dev -- docs` - launch a specific workspace dev surface through the root helper

## Workspace Boundaries

- Put native browser orchestration in `crates/shell`, `crates/engine`, `crates/memory`, `crates/ui`, and `crates/runtime-api`.
- Put app-level browser startup in `apps/browser`.
- Put reusable brand, schema, and content contracts in `packages/*`.
- Put public-facing product and developer content in the Astro site at `apps/docs` and long-form project references in `docs/`.
