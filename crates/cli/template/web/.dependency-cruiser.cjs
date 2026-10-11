// The Dependency Rule of the frontend — the same one the Rust crates follow:
//
//   presentation → infrastructure → application → domain
//
// `bun run lint:arch` fails on any import that points the other way.
module.exports = {
  forbidden: [
    {
      name: "domain-imports-nothing",
      comment: "domain/ is plain TypeScript: no npm package, no other layer.",
      severity: "error",
      from: { path: "^src/domain/" },
      to: { pathNot: "^src/domain/" },
    },
    {
      name: "application-imports-domain-only",
      comment: "A use case takes its ports as arguments; it never reaches an adapter or a screen.",
      severity: "error",
      from: { path: "^src/application/" },
      to: { pathNot: "^src/(application|domain)/" },
    },
    {
      name: "infrastructure-implements-domain-ports",
      comment: "Adapters know the domain, never the screens or the use cases.",
      severity: "error",
      from: { path: "^src/infrastructure/" },
      to: { path: "^src/(application|presentation)/" },
    },
    {
      name: "only-the-composition-root-imports-infrastructure",
      comment:
        "Screens reach adapters through useDependencies(); only core/generated/dependencies.ts builds them.",
      severity: "error",
      from: {
        path: "^src/presentation/",
        pathNot: "^src/presentation/core/generated/dependencies\\.ts$",
      },
      to: { path: "^src/infrastructure/" },
    },
    {
      name: "test-support-never-reaches-an-adapter",
      comment:
        "Mothers and fakes stand in for adapters: they build domain values, never call the API.",
      severity: "error",
      from: { path: "^src/test-support/" },
      to: { path: "^src/(application|infrastructure)/" },
    },
    {
      name: "only-tests-import-test-support",
      comment: "Fakes and mothers are for tests; the app never ships them.",
      severity: "error",
      from: { path: "^src/", pathNot: ["^src/test-support/", "\\.test\\.tsx?$"] },
      to: { path: "^src/test-support/" },
    },
    {
      name: "no-circular",
      severity: "error",
      from: {},
      to: { circular: true },
    },
  ],
  options: {
    doNotFollow: { path: "node_modules" },
    tsPreCompilationDeps: true,
    tsConfig: { fileName: "tsconfig.json" },
  },
};
