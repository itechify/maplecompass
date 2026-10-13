import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { byId, span } from "./dom";
import { signed, whole } from "./figures";

/** Mirror of `minerva_core::ListedPrice`: `quantity` of the item for `mesos`. */
type ListedPrice = { quantity: number; mesos: number };

/** Mirror of `minerva_core::DealItem`. */
type DealItem = { id: string; name: string };

/** Mirror of `minerva_core::DealBadge`: one listing's badge, level with it (physical px
 * from the window's top), and what it says; `covered`, what was read there before the
 * pointer or a tooltip covered it. */
export type DealBadge = { row: number; top: number; height: number; covered: boolean } & (
  | { verdict: "deal" | "noDeal"; item: DealItem; listed: ListedPrice; percent: number; lineMesos: number }
  | { verdict: "noPrice"; item: DealItem; listed: ListedPrice }
  | { verdict: "unknown" }
);

/** What the shell sends: the badges (none while the window draws nothing), and the Deal
 * Line's name for their words. */
type BadgesView = { badges: DealBadge[]; dealLine: string | null };

const list = byId("badges");

/** How far off the Deal Line a price is, as the badge says it: "−32%", "+12%", "0%"; a deal
 * under a percent off is "−<1%". */
function off(percent: number, deal: boolean): string {
  if (percent === 0) return deal ? "−<1%" : "0%";
  return percent > 0 ? `+${percent}%` : signed(`${percent}%`);
}

/** The price of one, as the game writes prices: whole mesos, or to a tenth. */
function each({ quantity, mesos }: ListedPrice): string {
  const one = mesos / quantity;
  return Number.isInteger(one) ? whole(one) : one.toLocaleString("en-US", { maximumFractionDigits: 1 });
}

/** The badge's words, and what it says in full for its tooltip. */
function words(badge: DealBadge, line: string): [string, string] {
  switch (badge.verdict) {
    case "deal":
      return [
        `${off(badge.percent, true)} · ${whole(badge.lineMesos)}`,
        `${badge.item.name}: ${each(badge.listed)} mesos each, a deal against ${line} (${whole(badge.lineMesos)}). Click for its card.`,
      ];
    case "noDeal":
      return [
        off(badge.percent, false),
        `${badge.item.name}: ${each(badge.listed)} mesos each, no deal against ${line} (${whole(badge.lineMesos)}). Click for its card.`,
      ];
    case "noPrice":
      return ["no price", `${badge.item.name}: ${line} has no price for it. Click to set one on its card.`];
    case "unknown":
      return ["?", "Couldn't read this listing for sure, so it isn't judged."];
  }
}

function badgeItem(badge: DealBadge, line: string): HTMLLIElement {
  const row = document.createElement("li");
  row.className = "deal-slot";
  // The core places badges in physical px; the page lays out in CSS px.
  const scale = window.devicePixelRatio || 1;
  row.style.top = `${badge.top / scale}px`;
  row.style.height = `${badge.height / scale}px`;
  const [text, said] = words(badge, line);
  const full = !badge.covered
    ? said
    : badge.verdict === "unknown"
      ? "Covered by the pointer or a tooltip, and not read before it was."
      : `Covered by the pointer or a tooltip: this is what it read before. ${said}`;
  const opens = badge.verdict !== "unknown";
  const el = document.createElement(opens ? "button" : "span");
  el.className = `deal-badge ${badge.verdict}`;
  el.classList.toggle("covered", badge.covered);
  el.title = full;
  el.setAttribute("aria-label", full);
  if (el instanceof HTMLButtonElement) {
    el.type = "button";
    el.dataset.row = String(badge.row);
  }
  // Covered, its verdict stays, with the mark that it isn't being read now.
  if (badge.covered && badge.verdict !== "unknown") el.append(span("deal-covered", "?"));
  el.append(span("deal-words", text));
  row.append(el);
  return row;
}

let shown = "";
function render(view: BadgesView) {
  // Without badges the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", view.badges.length === 0);
  const key = JSON.stringify([view, window.devicePixelRatio]);
  if (key === shown) return;
  shown = key;
  const line = view.dealLine ?? "the Deal Line";
  list.replaceChildren(...view.badges.map((badge) => badgeItem(badge, line)));
}

// A click on a badge shows its item's card. The window never takes the keyboard: it can't
// be activated.
list.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".deal-badge[data-row]") : null;
  if (button?.dataset.row) invoke("deal_badge_open", { row: Number(button.dataset.row) });
});

let last: BadgesView | null = null;
listen<BadgesView>("deal-badges-state", (e) => {
  last = e.payload;
  render(e.payload);
});
// A move to a display of another scale lays the badges out afresh.
window.addEventListener("resize", () => last && render(last));
// The badges may have been sent before the page was listening.
invoke<BadgesView | null>("deal_badges_view").then((view) => {
  if (view) {
    last = view;
    render(view);
  }
});
