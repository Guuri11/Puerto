# Puerto — website

The site at https://puerto-rs.vercel.app: the home page and the documentation (Astro + Starlight).

```bash
pnpm install
pnpm dev              # copies the ADRs and the changelog from the repo, then serves
pnpm test:e2e         # build + Playwright on :4347
```

Most of what the site shows is read, not written:

- `src/lib/repo.ts` reads `../Cargo.toml` and `../docs/adr` at build time.
- `scripts/sync-repo-docs.mjs` turns the ADRs, the changelog, the CLI's `--help` and the
  generated Makefiles into docs pages (git-ignored: edit the sources).
- `pnpm harvest` runs the real `puerto` CLI over 66 combinations of flags and entities for the
  home page's terminal; `pnpm capture:compiler` captures a real `cargo check` error.

After a Puerto release, run `make site/data` from the repository root.

Internal notes (Spanish): `CLAUDE.md`, `brief.md`, `DESIGN.md`, `PENDIENTE.md`, `CAMBIOS.md`.
