// The Worker's tests (`pnpm site:test`), run in workerd against a local D1 with the
// migrations applied, a made-up ceilings table and a made-up maintainer token.
import path from "node:path";
import { cloudflareTest, readD1Migrations } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

export default defineConfig(async () => {
  const migrations = await readD1Migrations(path.join(import.meta.dirname, "../migrations"));
  return {
    plugins: [
      cloudflareTest({
        wrangler: { configPath: path.join(import.meta.dirname, "../wrangler.jsonc") },
        miniflare: {
          bindings: {
            TEST_MIGRATIONS: migrations,
            ADMIN_TOKEN: "made-up-maintainer-token",
            // Made-up maps' solo ceilings, in place of osms-ceilings.json.
            TEST_CEILINGS: { "100000000": 100_000, "100000001": 2_000_000 },
          },
        },
      }),
    ],
    // Run from the repo root with --config, Vite's root would be the whole repo.
    test: { root: import.meta.dirname, setupFiles: [path.join(import.meta.dirname, "test/apply-migrations.ts")] },
  };
});
