import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dayText } from "./dates";
import { byId, span } from "./dom";

/** Mirror of `compass_core::ShopPrice`. */
type ShopPrice = { status: "sells"; mesos: number } | { status: "notSold" } | { status: "cash" } | { status: "unknown" };

/** Mirror of `compass_core::Market`. */
type Market =
  | { status: "loading" }
  | { status: "sold"; mesos: number; sales: number; page: string; fromUnixMs: number | null }
  | { status: "noSales"; page: string | null; fromUnixMs: number | null }
  | { status: "unavailable" };

/** Mirror of `compass_core::QuestReward`. */
type QuestReward = { status: "always" } | { status: "chance"; hundredths: number } | { status: "pick" } | { status: "sometimes" };

/** Mirrors of `compass_core::ItemSource` and `compass_core::ItemUse`. */
type ItemSource = { kind: "quest"; name: string; reward: QuestReward } | { kind: "recipe"; discipline: string; level: number };
type ItemUse = { kind: "quest"; name: string; count: number } | { kind: "recipe"; makes: string; count: number };

/** Mirror of `compass_core::Listed`. */
type Listed<T> = { shown: T[]; more: number };

/** Mirror of `compass_core::ItemCard`. */
type ItemCard = {
  id: string;
  name: string;
  shop: ShopPrice;
  market: Market;
  comesFrom: Listed<ItemSource>;
  usedFor: Listed<ItemUse>;
  savedCopyFromUnixMs: number | null;
  credit: string;
};

/** Mirror of `compass_core::ItemCardChoice`. */
type ItemCardChoice = { id: string; kind: string; icon: string | null };

/** Mirror of `compass_core::ShownItemCard`. */
type ShownItemCard = { card: number } & (
  | ({ status: "card" } & ItemCard)
  | { status: "choose"; name: string; items: ItemCardChoice[] }
);

/** What the shell sends: the card, and whether it shows at all. */
type CardView = { shown: boolean; card: ShownItemCard | null };

const cardEl = byId("card");
const title = byId("title");
const body = byId("body");

const mesos = (amount: number) => `${amount.toLocaleString("en-US")} mesos`;
const dated = (unixMs: number) => `Prices from ${dayText(new Date(unixMs), false)}`;

/** One figure: its label at the left, its value right-aligned, as the Mob Card sets them. */
function figure(label: string, value: Node | string, quiet = false): HTMLDivElement {
  const row = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = label;
  const dd = document.createElement("dd");
  dd.append(value);
  dd.classList.toggle("quiet", quiet);
  row.append(dt, dd);
  return row;
}

/** What a shop pays, or why it says nothing. */
function shopFigure(shop: ShopPrice): HTMLDivElement {
  switch (shop.status) {
    case "sells":
      return figure("Shop pays", mesos(shop.mesos));
    case "notSold":
      return figure("Shop pays", "Can't be sold", true);
    case "cash":
      return figure("Shop pays", "Cash item", true);
    case "unknown":
      return figure("Shop pays", "Not known", true);
  }
}

/** A link on the card: Portal Blue under the pointer, as a mob's name in the panel. */
function link(text: string, label: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "item-card-link";
  button.textContent = text;
  button.title = label;
  button.addEventListener("click", onClick);
  return button;
}

/** What players pay for it on BasilMarket, from how many sales; a link to its page there. */
function marketFigure(market: Market): HTMLDivElement {
  const openMarket = () => invoke("item_card_open_market");
  switch (market.status) {
    case "loading":
      return figure("Market", "Loading…", true);
    case "unavailable":
      return figure("Market", "Unavailable", true);
    case "sold": {
      const value = document.createDocumentFragment();
      value.append(link(mesos(market.mesos), "See its prices on BasilMarket", openMarket));
      value.append(span("sales", ` · ${market.sales} ${market.sales === 1 ? "sale" : "sales"}`));
      return figure("Market", value);
    }
    case "noSales": {
      const text = "No sales seen yet";
      return market.page
        ? figure("Market", link(text, "See it on BasilMarket", openMarket), true)
        : figure("Market", text, true);
    }
  }
}

/** A quest's chance of giving the item, as OSMS gives it: "7.14%". */
function chance(reward: QuestReward): string {
  switch (reward.status) {
    case "always":
      return "";
    case "chance":
      return `${Number((reward.hundredths / 100).toFixed(2))}%`;
    case "pick":
      return "a pick";
    case "sometimes":
      return "sometimes";
  }
}

/** One row of a list: what it is, its name, and how many or how likely at the right. */
function entry(tag: string, name: string, aside: string): HTMLLIElement {
  const row = document.createElement("li");
  const tagEl = span("tag", tag);
  row.append(tagEl, span("what", name));
  if (aside) row.append(span("aside", aside));
  row.title = aside ? `${name} (${aside})` : name;
  return row;
}

const sourceRow = (source: ItemSource) =>
  source.kind === "quest"
    ? entry("Quest", source.name, chance(source.reward))
    : entry("Craft", `${source.discipline}, level ${source.level}`, "");

const useRow = (used: ItemUse) =>
  used.kind === "quest" ? entry("Quest", used.name, `×${used.count}`) : entry("Craft", used.makes, `×${used.count}`);

/** A list on the card, under its heading: the first few, then how many more. Left out
 * with nothing in it. */
function list<T>(heading: string, listed: Listed<T>, row: (item: T) => HTMLLIElement): HTMLElement[] {
  if (listed.shown.length === 0) return [];
  const section = document.createElement("section");
  section.className = "item-card-list";
  const h2 = document.createElement("h2");
  h2.textContent = heading;
  const ul = document.createElement("ul");
  ul.append(...listed.shown.map(row));
  section.append(h2, ul);
  if (listed.more > 0) section.append(span("more", `and ${listed.more} more`));
  return [section];
}

/** One item's card. */
function renderCard(card: ItemCard) {
  title.replaceChildren(link(card.name, `Open ${card.name} on MeowDB`, () => invoke("item_card_open_meowdb")));
  const prices = document.createElement("dl");
  prices.className = "item-card-prices";
  prices.append(shopFigure(card.shop), marketFigure(card.market));
  const parts: Node[] = [prices];
  const from = card.market.status === "sold" || card.market.status === "noSales" ? card.market.fromUnixMs : null;
  if (from !== null) parts.push(span("item-card-dated", dated(from)));
  parts.push(...list("Comes from", card.comesFrom, sourceRow), ...list("Used for", card.usedFor, useRow));
  if (card.savedCopyFromUnixMs !== null) {
    const note = document.createElement("p");
    note.className = "item-card-note";
    note.textContent = `OSMS Data Explorer unreachable — item data from ${dayText(new Date(card.savedCopyFromUnixMs), true)}`;
    parts.push(note);
  }
  const credit = document.createElement("p");
  credit.className = "item-card-credit";
  credit.textContent = card.credit;
  parts.push(credit);
  body.replaceChildren(...parts);
}

/** The items sharing the name read, for the player to pick theirs by its picture. */
function renderChoice(name: string, items: ItemCardChoice[]) {
  title.replaceChildren(span("item-card-ask", `Which ${name}?`));
  const choices = document.createElement("ol");
  choices.className = "item-card-choices";
  choices.append(
    ...items.map((item) => {
      const row = document.createElement("li");
      const button = document.createElement("button");
      button.type = "button";
      button.className = "item-card-choice";
      button.title = `The ${item.kind} ${name}`;
      const icon = document.createElement("span");
      icon.className = "item-card-icon";
      if (item.icon) {
        const img = document.createElement("img");
        img.src = item.icon;
        img.alt = "";
        icon.append(img);
      }
      button.append(icon, span("kind", item.kind));
      button.addEventListener("click", () => invoke("item_card_pick", { id: item.id }));
      row.append(button);
      return row;
    }),
  );
  const hint = document.createElement("p");
  hint.className = "item-card-hint";
  hint.textContent = "Pick the one whose picture the game's tooltip shows.";
  body.replaceChildren(hint, choices);
}

let shownKey = "";
function render({ shown, card }: CardView) {
  // Not showing, the window draws nothing (and the shell lets clicks through it).
  document.body.classList.toggle("hidden", !shown || !card);
  if (!shown || !card) {
    shownKey = "";
    return;
  }
  const key = JSON.stringify(card);
  if (key === shownKey) return;
  shownKey = key;
  if (card.status === "card") renderCard(card);
  else renderChoice(card.name, card.items);
  reportHeight();
}

// The shell sizes the window to the card, as the overlay does the panel.
let reportedHeight = 0;
function reportHeight() {
  const height = Math.ceil(cardEl.getBoundingClientRect().height);
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("item_card_height", { height });
  }
}
window.addEventListener("resize", reportHeight);
// The height depends on the font, and on pictures arriving: measure again once in.
document.fonts.ready.then(reportHeight);
new ResizeObserver(reportHeight).observe(cardEl);

// The window takes clicks only while a card shows, and never the keyboard: it can't be
// activated.
byId("close").addEventListener("click", () => invoke("item_card_close"));

listen<CardView>("item-card-state", (e) => render(e.payload));
// The card may have been sent before the page was listening.
invoke<CardView | null>("item_card_view").then((view) => {
  if (view) render(view);
});
