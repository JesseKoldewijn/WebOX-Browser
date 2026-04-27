# Monorepo Architecture

The repository is structured to keep the browser application, embeddable runtime, shared packages, and docs site in one place.

## Areas

- `apps/browser`: binary entrypoint for the standalone browser shell
- `apps/docs`: Astro-powered docs and marketing site with SolidJS components for interactivity
- `crates/*`: Rust modules for browser host responsibilities
- `packages/*`: reusable TypeScript packages for shared branding and contracts

## Ownership Rules

- Browser lifecycle logic belongs in Rust crates, not the docs site.
- Shared contracts and tokens should move into `packages/*` once reused by more than one surface.
- Validation fixtures live under `validation/` so they are reusable by future browser automation.
