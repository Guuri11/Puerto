// The frontend is the presentation layer (ADR-0025 of Puerto): containers call the API, views
// take props. `bun run lint:arch` fails on any import that breaks it.
module.exports = {
  forbidden: [
    {
      name: "views-take-props",
      comment:
        "A view renders what its container hands it: no API client, no dependencies, no data hooks (types are fine).",
      severity: "error",
      from: { path: "^src/features/[^/]+/views/" },
      to: {
        path: "^src/(api/|dependencies\\.tsx$|generated/|shared/use-async\\.ts$)",
        dependencyTypesNot: ["type-only"],
      },
    },
    {
      name: "only-the-composition-root-builds-the-client",
      comment:
        "Screens reach the API through useDependencies(), so a test hands them a fake: only generated/dependencies.ts and test-support/ build it.",
      severity: "error",
      from: {
        path: "^src/",
        pathNot: ["^src/generated/dependencies\\.ts$", "^src/test-support/"],
      },
      to: { path: "^src/api/client\\.ts$", dependencyTypesNot: ["type-only"] },
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
