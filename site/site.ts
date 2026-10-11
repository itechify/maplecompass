/// <reference types="vite/client" />
// The site's one behaviour: the tab of the part being read is raised, as the game raises
// the open tab, following the scroll. Without it the page still reads whole, and the tabs
// still jump to their parts.

const head = document.querySelector<HTMLElement>(".window-head")!;
const strip = document.querySelector<HTMLElement>(".tabs")!;
const tabs = [...strip.querySelectorAll<HTMLAnchorElement>("a.tab")];
const pages = tabs.map((tab) => document.getElementById(tab.hash.slice(1))!);

// A jump to a heading lands below the window's head, however tall it wraps.
new ResizeObserver(() => {
  document.documentElement.style.setProperty("--head", `${head.offsetHeight}px`);
}).observe(head);

let raised = -1;

/** Raise the tab of the last part whose top has come up to the head. */
function raise() {
  const line = head.getBoundingClientRect().bottom + 48;
  let current = 0;
  pages.forEach((page, i) => {
    if (page.getBoundingClientRect().top <= line) current = i;
  });
  // Scrolled to the end, the last part is the one being read, however short it is.
  const end = document.documentElement.scrollHeight - innerHeight - 2;
  if (scrollY > 0 && scrollY >= end) current = pages.length - 1;
  if (current === raised) return;
  raised = current;
  tabs.forEach((tab, i) => {
    if (i === current) tab.setAttribute("aria-current", "location");
    else tab.removeAttribute("aria-current");
  });
  // On a narrow screen the strip scrolls sideways: keep the raised tab in it.
  const tab = tabs[current];
  if (tab.offsetLeft < strip.scrollLeft) strip.scrollLeft = tab.offsetLeft - 12;
  else if (tab.offsetLeft + tab.offsetWidth > strip.scrollLeft + strip.clientWidth) {
    strip.scrollLeft = tab.offsetLeft + tab.offsetWidth - strip.clientWidth + 12;
  }
}

addEventListener("scroll", raise, { passive: true });
addEventListener("resize", raise);
raise();

// A screenshot that doesn't load leaves a dashed box saying what belongs there. While
// developing, that is a shot not taken yet; the build refuses to publish without it.
for (const img of document.querySelectorAll<HTMLImageElement>(".shot img")) {
  const figure = img.closest("figure")!;
  const missing = () => {
    figure.dataset.missing = import.meta.env.DEV
      ? `Not taken yet: site/${img.getAttribute("src")?.replace(/^\.\//, "")}. See site/screenshots/README.md.\n\n${img.alt}`
      : img.alt;
    figure.classList.add("missing");
  };
  if (img.complete && img.naturalWidth === 0) missing();
  else img.addEventListener("error", missing);
}
