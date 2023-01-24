_default:
  @just --list

sync-api-types:
  cd web && pnpm generate-api-types

run-api:
  cargo run --release

dev-web:
  cd web && pnpm dev:svelte
