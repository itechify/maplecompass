import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import pkg from "../package.json" with { type: "json" };
import { hotkeyRows } from "./hotkeys.ts";

// The MapleCompass site: one page for players, published on GitHub Pages from a release's
// tag (.github/workflows/site.yml). `pnpm site:dev` serves it, `pnpm site:build` builds it
// into site/dist.

/** Where the site is published, for the links that must be absolute (Open Graph). */
const SITE_URL = "https://itechify.github.io/maplecompass/";
const RELEASES = "https://github.com/itechify/maplecompass/releases";

/**
 * The page's placeholders, filled at build: the version it describes (package.json's, which
 * at a release's tag is the release's), that version's downloads, and the hotkey table.
 */
function fillPlaceholders(): Plugin {
  const version = pkg.version;
  const values: Record<string, string> = {
    "@VERSION@": version,
    "@SETUP_URL@": `${RELEASES}/download/v${version}/MapleCompass_${version}_x64-setup.exe`,
    "@RELEASE_URL@": `${RELEASES}/tag/v${version}`,
    "@SITE_URL@": SITE_URL,
    "@HOTKEY_ROWS@": hotkeyRows(),
  };
  return {
    name: "maplecompass-site:placeholders",
    transformIndexHtml: {
      order: "pre",
      handler: (html) => html.replace(/@[A-Z_]+@/g, (token) => values[token] ?? token),
    },
  };
}

/**
 * The screenshots are taken by hand in the game (site/screenshots/README.md), so the build
 * fails while one is missing rather than publish the page without it. Vite leaves the
 * address of a picture it couldn't find as written. Open Graph and Twitter cards then get
 * the hero shot's absolute address, known only once the build has named the file.
 */
function screenshots(): Plugin {
  return {
    name: "maplecompass-site:screenshots",
    apply: "build",
    transformIndexHtml: {
      order: "post",
      handler(html) {
        const missing = [...html.matchAll(/<img[^>]*\bsrc="\.?\/?(screenshots\/[^"]+)"/g)].map((m) => `site/${m[1]}`);
        if (missing.length > 0) {
          throw new Error(`site: not taken yet: ${missing.join(", ")} (see site/screenshots/README.md)`);
        }
        const hero = /<img[^>]*\bid="hero-shot"[^>]*\bsrc="([^"]+)"/.exec(html)?.[1];
        if (!hero) throw new Error("site: the hero shot (img#hero-shot) is missing from the page");
        // The address as Vite writes it from the site's root (it fills in the file's name
        // last), after the site's origin.
        return html.replace(/@CARD_IMAGE@/g, new URL(SITE_URL).origin + hero);
      },
    },
  };
}

/**
 * Each screenshot's own size on its <img>, read from the PNG's header, so the page keeps
 * its room before the picture arrives and a crop shows at its full size.
 */
function screenshotSizes(): Plugin {
  return {
    name: "maplecompass-site:screenshot-sizes",
    transformIndexHtml: {
      order: "pre",
      handler: (html) =>
        html.replace(/<img\b([^>]*?)\bsrc="\.\/(screenshots\/[^"]+\.png)"/g, (tag, before: string, path: string) => {
          const file = fileURLToPath(new URL(`./${path}`, import.meta.url));
          if (!existsSync(file)) return tag;
          // A PNG's width and height are its IHDR chunk's first eight bytes, after the
          // eight-byte signature and the chunk's length and type.
          const header = readFileSync(file).subarray(16, 24);
          const size = `width="${header.readUInt32BE(0)}" height="${header.readUInt32BE(4)}" `;
          return `<img${before}${size}src="./${path}"`;
        }),
    },
  };
}

/**
 * The page names the app's theme and icon outside site/ (../src, ../assets), as the build
 * takes them. The dev server serves only its root, so it serves those from the repo.
 */
function serveAppFiles(): Plugin {
  return {
    name: "maplecompass-site:app-files",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use((req, _res, next) => {
        if (req.url && /^\/(src|assets)\//.test(req.url)) {
          const file = fileURLToPath(new URL(`..${req.url}`, import.meta.url)).replace(/\\/g, "/");
          req.url = `/maplecompass/@fs${file.startsWith("/") ? "" : "/"}${file}`;
        }
        next();
      });
    },
  };
}

/**
 * The app's tokens without its typeface: theme.css names the TTFs the app bundles, and the
 * site serves lighter WOFF2 copies of the same files instead (site/fonts.css).
 */
function themeWithoutFonts(): Plugin {
  return {
    name: "maplecompass-site:theme-tokens",
    enforce: "pre",
    transform(code, id) {
      if (!id.split("?")[0].endsWith("/src/theme.css")) return;
      return code.replace(/@font-face\s*\{[^}]*\}/g, "");
    },
  };
}

export default defineConfig({
  root: "site",
  base: "/maplecompass/",
  publicDir: false,
  plugins: [fillPlaceholders(), screenshotSizes(), screenshots(), serveAppFiles(), themeWithoutFonts()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // A screenshot keeps a file of its own, however small, so the cards can address it.
    assetsInlineLimit: (file) => (file.includes("/screenshots/") ? false : undefined),
  },
});
