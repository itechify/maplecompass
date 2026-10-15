import { applyD1Migrations } from "cloudflare:test";
import { env } from "cloudflare:workers";

// Setup files may run more than once; applying the migrations again does nothing.
await applyD1Migrations(env.DB, env.TEST_MIGRATIONS);
