/** Map Marks: the emblem the game's Mini Map draws beside a map's title, for the area the
 *  map belongs to, which every window shows beside a map's name. */

/** Mirror of `minerva_core::MapMark`. */
export type MapMark = { name: string; image: string | null };

/** Mirror of the panel state's `mapMarks`: every mark, by OSMS's name for it, which each
 *  map names its own by. */
export type MapMarks = Record<string, MapMark>;

/** The mark of a map that names `key` as its own (none: a map without one), to set before
 *  its name: its picture once it's in, named for its area. Until then, or for a map
 *  without one, an empty slot of its size, so names in a list line up. */
export function mapMark(marks: MapMarks, key: string | null | undefined): HTMLImageElement {
  const img = document.createElement("img");
  img.className = "map-mark";
  img.draggable = false;
  showMark(img, marks, key);
  return img;
}

/** Show the mark of a map that names `key` as its own in `img`, kept from one render to
 *  the next (the title's). Whether the map has one. */
export function showMark(img: HTMLImageElement, marks: MapMarks, key: string | null | undefined): boolean {
  const mark = key ? marks[key] : undefined;
  img.alt = mark?.name ?? "";
  if (!mark?.image) img.removeAttribute("src");
  else if (img.getAttribute("src") !== mark.image) img.src = mark.image;
  return !!mark;
}
