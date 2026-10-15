// The Map Leaderboards pages' tests (`pnpm site:test`): what a page shows, worked out from
// a link, the map names fixture and the API's answers (leaderboards.ts), in plain Node.
import { defineConfig } from "vitest/config";

export default defineConfig({
  // Run from the repo root with --config, Vite's root would be the whole repo.
  test: { root: import.meta.dirname, include: ["test/**/*.test.ts"] },
});
