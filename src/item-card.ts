import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { dayText } from "./dates";
import { byId, span } from "./dom";

/** Mirror of `compass_core::ShopPrice`. */
type ShopPrice = { status: "sells"; mesos: number } | { status: "notSold" } | { status: "cash" } | { status: "unknown" };

/** Mirror of `compass_core::MarketPrice`: a price the player keeps, and when they set it. */
type MarketPrice = { mesos: number; setAtUnixMs?: number };

/** Mirror of `compass_core::MarketEdit`: the market price's field, open. */
type MarketEdit = { notAPrice: string | null };

/** Mirror of `compass_core::ItemCard`. */
type ItemCard = {
  id: string;
  name: string;
  shop: ShopPrice;
  market: MarketPrice | null;
  marketEdit: MarketEdit | null;
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

/** A light icon button in the card's body: `path` drawn as the panel's icons are. */
function iconButton(path: string, label: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button light";
  button.setAttribute("aria-label", label);
  button.title = label;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("aria-hidden", "true");
  const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
  line.setAttribute("d", path);
  svg.append(line);
  button.append(svg);
  button.addEventListener("click", onClick);
  return button;
}

/** MesoWatch, which has no page per item: the player looks the item up there. */
const mesoWatchButton = () =>
  iconButton("M9 3h4v4M13 3L7.5 8.5M11.5 9.5V13h-8.5V4.5H6.5", "Look it up on MesoWatch", () =>
    invoke("item_card_open_mesowatch"),
  );

/** The market price's field, open: what the player typed (or the price kept), Enter to
 * keep it, Esc to leave it as it was. Leaving the field keeps what is typed. */
let fieldDone = false;
function marketField(card: ItemCard): HTMLElement[] {
  const input = document.createElement("input");
  input.type = "text";
  input.className = "item-card-input";
  input.inputMode = "decimal";
  input.autocomplete = "off";
  input.spellcheck = false;
  input.placeholder = "450,000 or 450k";
  input.setAttribute("aria-label", `Your market price for ${card.name}`);
  input.value = card.marketEdit?.notAPrice ?? (card.market ? card.market.mesos.toLocaleString("en-US") : "");
  fieldDone = false;
  const finish = (command: "item_card_enter_market" | "item_card_cancel_market") => {
    if (fieldDone) return;
    fieldDone = true;
    invoke(command, command === "item_card_enter_market" ? { typed: input.value } : {});
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") finish("item_card_enter_market");
    if (e.key === "Escape") finish("item_card_cancel_market");
  });
  input.addEventListener("blur", () => {
    if (input.isConnected) finish("item_card_enter_market");
  });
  const row = document.createElement("div");
  row.className = "item-card-market editing";
  row.append(span("label", "Market"), input, mesoWatchButton());
  const note = document.createElement("p");
  const problem = card.marketEdit?.notAPrice;
  note.className = problem ? "item-card-hint problem" : "item-card-hint";
  note.textContent = problem
    ? `“${problem}” isn't a price. Try 450,000, 450k or 1.2m.`
    : "Enter keeps it · Esc cancels · empty clears";
  queueMicrotask(() => {
    input.focus();
    input.select();
  });
  return [row, note];
}

/** The market price the player keeps (the day they set it under the pointer), and buttons
 * to change it and to look it up on MesoWatch. */
function marketRow(card: ItemCard): HTMLElement {
  const row = document.createElement("div");
  row.className = "item-card-market";
  const edit = () => invoke("item_card_edit_market");
  const setOn = card.market?.setAtUnixMs;
  const value = card.market
    ? link(mesos(card.market.mesos), `Your price${setOn === undefined ? "" : `, set ${dayText(new Date(setOn), false)}`}. Click to change it`, edit)
    : link("Not set", "Set your market price", edit);
  value.classList.toggle("quiet", !card.market);
  value.classList.add("value");
  row.append(span("label", "Market"), value);
  row.append(
    iconButton("M3 13l.8-3.2L10.6 3l2.4 2.4-6.8 6.8L3 13zM9.3 4.3l2.4 2.4", "Set your market price", edit),
    mesoWatchButton(),
  );
  return row;
}

/** One item's card. */
function renderCard(card: ItemCard) {
  title.replaceChildren(link(card.name, `Open ${card.name} on MeowDB`, () => invoke("item_card_open_meowdb")));
  const prices = document.createElement("dl");
  prices.className = "item-card-prices";
  prices.append(shopFigure(card.shop));
  const box = document.createElement("div");
  box.className = "item-card-box";
  box.append(prices, ...(card.marketEdit ? marketField(card) : [marketRow(card)]));
  const parts: Node[] = [box];
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
  const box = document.createElement("div");
  box.className = "item-card-box";
  box.append(choices);
  body.replaceChildren(hint, box);
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

// The shell sizes the window to the card, up to its most; past that the card scrolls under
// its bar. The card is no taller than the window, so it is measured without that limit, or
// the window could never grow.
let reportedHeight = 0;
function reportHeight() {
  cardEl.classList.add("measuring");
  const height = Math.ceil(cardEl.getBoundingClientRect().height);
  cardEl.classList.remove("measuring");
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("item_card_height", { height });
  }
}
window.addEventListener("resize", reportHeight);
// The height depends on the font, and on pictures arriving: measure again once in.
document.fonts.ready.then(reportHeight);
new ResizeObserver(reportHeight).observe(cardEl);

// Dragged by its bar, never by its button. The window takes clicks only while a card
// shows, and the keyboard only while its market price's field is open.
document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onBar = !!target?.closest(".drag-handle") && !target?.closest("button");
  if (e.button === 0 && onBar) getCurrentWindow().startDragging();
});
byId("close").addEventListener("click", () => invoke("item_card_close"));

listen<CardView>("item-card-state", (e) => render(e.payload));
// The card may have been sent before the page was listening.
invoke<CardView | null>("item_card_view").then((view) => {
  if (view) render(view);
});
