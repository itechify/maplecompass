//! The Player Shop reader (ADR 0014): finds an open Player Shop's item list in a capture of
//! the game's client area and reads each row's name and price, for the Deal Finder.
//!
//! The game draws the shop's UI at a fixed pixel size, wherever on the client area the
//! window is and whatever the client area's size:
//!
//! 1. Each listing is a light blue-grey band (202,213,223) exactly [`BAND_W`] px wide, with
//!    its item's name on one line and "1 for 999 mesos" on the line [`PRICE_BELOW`] px
//!    below it. A band row is a scanline run of that colour [`BAND_W`] px long; the list is
//!    where many such runs start at the same column. Live captures show the game's colours
//!    a little darker than screenshots do (`prep.rs`), so the band is matched at any
//!    brightness, as the minimap's panel is.
//! 2. The text is black, blended into the band where it doesn't cover a pixel fully, so
//!    every pixel of a listing's lines lies between the band's colour and black. A pixel
//!    that doesn't (the game's pointer, a window of ours) means the line is covered: the
//!    row is unreadable, never misread.
//! 3. A sold-out listing is greyed: its band isn't the listing colour, so it isn't read.
//!    An empty slot is a band with no text. The listing the player selected is orange
//!    (251,167,46) from 7 px into its band, its text black as on the others, and is read
//!    as they are.
//! 4. A slot where a listing was read before, at the same spot, that now has none and holds
//!    pixels the shop never draws, is covered (by the pointer, or the game's tooltip by it):
//!    it is reported as such, so what it read before can stand, marked as covered.
//! 5. ocrs reads every line at once, black on white, enlarged twice over. A price counts
//!    only in the game's exact shape: "<quantity> for <mesos> mesos", the mesos with their
//!    commas where the game puts them.

use crate::Frame;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::prep::{PAD, Rect};
use crate::recogniser::{Recogniser, frame_to_rgb};
use image::{GrayImage, Luma, RgbImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A capture of the game's whole client area for the Deal Finder.
#[derive(Debug, Clone)]
pub struct ShopCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen: the client area's corner.
    pub origin: ScreenPoint,
}

/// What a Deal Finder capture reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopReading {
    /// The Player Shop's item list, if one shows.
    pub shop: Option<ShopList>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// A Player Shop's item list, as it shows on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopList {
    /// Where the list's listings are on screen, from the first one's top to the last one's
    /// bottom, the item pictures' column included: what a Bug Report keeps of the capture.
    pub area: ScreenRect,
    /// Its listings that aren't sold out, top to bottom.
    pub rows: Vec<ShopRow>,
}

/// One listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopRow {
    /// Where the listing is on screen: its band, both lines.
    pub area: ScreenRect,
    /// The item's name as read; `None`: unreadable or covered.
    pub name: Option<String>,
    /// Its price as read; `None`: unreadable, covered, or not in the game's shape.
    pub price: Option<ListedPrice>,
    /// Something covers it, so neither was read: the pointer, or the game's tooltip by it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub covered: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A listing's price: `quantity` of the item for `mesos`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedPrice {
    pub quantity: u64,
    pub mesos: u64,
}

/// Reads Deal Finder captures with the bundled recogniser (ADR 0002), which it shares with
/// the other readers. It remembers what each line it read said, so a shop left open is
/// read again without ocrs.
pub struct PlayerShopReader {
    recogniser: Recogniser,
    /// What each line read as, by its ink.
    read: HashMap<Vec<u8>, Option<String>>,
    /// The slots a listing was read in, while the list stays where it is: the column its
    /// bands start at, and each slot's top (frame px).
    slots: Option<(u32, Vec<u32>)>,
}

/// A listing's band is this many px wide.
const BAND_W: u32 = 253;
/// ...give or take this many.
const BAND_SLACK: u32 = 2;
/// The list is found where at least this many band rows start at one column: two
/// listings' worth of the rows above, between and below their text.
const MIN_BAND_ROWS: usize = 12;
/// The price line's top is this many px below the name line's, give or take
/// [`PRICE_SLACK`] (the letters' tops differ).
const PRICE_BELOW: u32 = 27;
const PRICE_SLACK: u32 = 3;
/// A listing's band starts this far above its price line's top (the "1" of "1 for", the
/// same height in every listing), and is this tall.
const BAND_ABOVE_PRICE: u32 = 33;
const BAND_H: u32 = 54;
/// The item pictures' column starts this far left of the band.
const PICTURE_LEFT: u32 = 58;
/// The band's colour, as screenshots show it.
const BAND_RGB: [f32; 3] = [202.0, 213.0, 223.0];
/// The selected listing's colour, as screenshots show it, and how far into the band it
/// starts (its edge is a blend with what is around it).
const SELECTED_RGB: [f32; 3] = [251.0, 167.0, 46.0];
const SELECTED_TOL: f32 = 10.0;
const SELECTED_INSET: u32 = 8;
/// Its text's anti-aliasing strays further from the orange-to-black line than the blue's.
const SELECTED_FOREIGN: f32 = 28.0;
/// The light rule between a listing's name and price, as screenshots show it, which a
/// descender may reach: on the band, and on the selected listing.
const RULE_RGB: [f32; 3] = [222.0, 223.0, 223.0];
const SELECTED_RULE_RGB: [f32; 3] = [255.0, 195.0, 96.0];
/// A pixel of the rule, or of a descender's ink blended into it, lies this close to the
/// rule-to-black line, and is no brighter than the rule (the pointer's white is).
const RULE_TOL: f32 = 16.0;
const RULE_BRIGHTEST: f32 = 1.04;
/// A slot is covered when at least this many of its pixels are none the shop draws.
const COVERED_MIN: usize = 15;
/// Two slots' tops this close are one slot's.
const SAME_SLOT: u32 = 3;
/// A pixel is the band's colour at its brightness within this much per channel...
const BAND_TOL: f32 = 6.0;
/// ...at any brightness in this range of the screenshots'.
const BAND_SCALE: std::ops::RangeInclusive<f32> = 0.8..=1.1;
/// A scanline is part of a listing's text when at least this share of it is the band.
const TEXT_ROW_BAND: f32 = 0.3;
/// A pixel is ink when it is at least this much of the way from the band to black.
const INK_MIN: f32 = 0.3;
/// A pixel further than this from the band-to-black line (per channel, roughly) isn't the
/// band or its text: something covers the listing.
const FOREIGN: f32 = 14.0;
/// Columns kept clear of the band's ends when looking at its text.
const EDGE: u32 = 2;
/// The lines are enlarged this many times before ocrs reads them.
const UPSCALE: u32 = 2;
/// How many lines' readings are remembered before they are forgotten all at once.
const REMEMBERED: usize = 512;

impl PlayerShopReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: HashMap::new(), slots: None }
    }

    pub fn read(&mut self, capture: &ShopCapture) -> ShopReading {
        let shop = frame_to_rgb(&capture.frame).and_then(|img| self.read_rgb(&img, capture.origin));
        ShopReading { shop, at_ms: capture.frame.at_ms }
    }

    fn read_rgb(&mut self, img: &RgbImage, origin: ScreenPoint) -> Option<ShopList> {
        let list = find_list(img)?;
        let listings = listings(img, &list);
        let on_screen = |r: Rect| ScreenRect { x: origin.x + r.x as i32, y: origin.y + r.y as i32, width: r.w, height: r.h };
        // Every clean line, read at once.
        let lines: Vec<&TextLine> = listings.iter().filter(|l| l.clean).flat_map(|l| [&l.name, &l.price]).collect();
        let texts = self.read_lines(img, &lines, list.scale);
        let mut texts = texts.into_iter();
        let band_area = |top: u32| Rect { x: list.x0, y: top, w: BAND_W, h: BAND_H };
        let mut rows: Vec<(u32, ShopRow)> = listings
            .iter()
            .map(|listing| {
                let (name, price) = if listing.clean {
                    let name = texts.next().flatten();
                    let price = texts.next().flatten().as_deref().and_then(parse_price);
                    (name, price)
                } else {
                    (None, None)
                };
                let top = listing.top();
                (top, ShopRow { area: on_screen(band_area(top)), name, price, covered: !listing.clean })
            })
            .collect();
        // The slots listings were read in at this spot; one of them with none now that
        // something covers is reported covered.
        let slots = match self.slots.take() {
            Some((x0, slots)) if x0 == list.x0 => slots,
            _ => Vec::new(),
        };
        let mut known = slots.clone();
        for &(top, _) in &rows {
            if !known.iter().any(|&slot| slot.abs_diff(top) <= SAME_SLOT) {
                known.push(top);
            }
        }
        for &slot in &slots {
            let read = rows.iter().any(|&(top, _)| top.abs_diff(slot) <= SAME_SLOT);
            if !read && covered(img, &list, slot) {
                rows.push((slot, ShopRow { area: on_screen(band_area(slot)), name: None, price: None, covered: true }));
            }
        }
        known.sort_unstable();
        self.slots = Some((list.x0, known));
        rows.sort_by_key(|&(top, _)| top);
        let rows = rows.into_iter().map(|(_, row)| row).collect();
        let x = list.x0.saturating_sub(PICTURE_LEFT);
        let area = Rect { x, y: list.y0, w: list.x0 + BAND_W - x, h: list.y1 - list.y0 };
        Some(ShopList { area: on_screen(area), rows })
    }

    /// Read `lines` of `img` (each its own ink, black on white), remembering what each
    /// one's ink read as: one entry per line, `None` where nothing was read.
    fn read_lines(&mut self, img: &RgbImage, lines: &[&TextLine], scale: f32) -> Vec<Option<String>> {
        let inks: Vec<GrayImage> = lines.iter().map(|line| line_ink(img, line, scale)).collect();
        let unknown: Vec<usize> = (0..inks.len()).filter(|&i| !self.read.contains_key(inks[i].as_raw())).collect();
        if !unknown.is_empty() {
            let read = self.recognise(&unknown.iter().map(|&i| &inks[i]).collect::<Vec<_>>());
            if self.read.len() + unknown.len() > REMEMBERED {
                self.read.clear();
            }
            for (&i, text) in unknown.iter().zip(read) {
                self.read.insert(inks[i].as_raw().clone(), text);
            }
        }
        inks.iter().map(|ink| self.read.get(ink.as_raw()).cloned().flatten()).collect()
    }

    /// ocrs's reading of each line's ink, stacked on one canvas: `None` where it read
    /// nothing, or for every line if it couldn't run.
    fn recognise(&self, inks: &[&GrayImage]) -> Vec<Option<String>> {
        let width = inks.iter().map(|ink| ink.width()).max().unwrap_or(0) + 2 * PAD;
        let height = inks.iter().map(|ink| ink.height() + PAD).sum::<u32>() + PAD;
        let mut canvas = GrayImage::from_pixel(width, height, Luma([255]));
        let mut rects = Vec::with_capacity(inks.len());
        let mut y = PAD;
        for ink in inks {
            image::imageops::replace(&mut canvas, *ink, PAD as i64, y as i64);
            rects.push(Rect { x: PAD * UPSCALE, y: y * UPSCALE, w: ink.width() * UPSCALE, h: ink.height() * UPSCALE });
            y += ink.height() + PAD;
        }
        let canvas = image::imageops::resize(&canvas, width * UPSCALE, height * UPSCALE, image::imageops::FilterType::Triangle);
        let Some(lines) = self.recogniser.read_lines(&canvas, &rects) else { return vec![None; inks.len()] };
        let text = |line: Option<&ocrs::TextLine>| {
            let text = line?.to_string();
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then_some(text)
        };
        (0..inks.len()).map(|i| text(lines.get(i).and_then(|line| line.as_ref()))).collect()
    }
}

/// Where the item list is: the column its bands start at, the rows from the first band row
/// to the last, and how bright the band is next to the screenshots' colour.
#[derive(Debug, Clone, Copy)]
struct List {
    x0: u32,
    y0: u32,
    y1: u32,
    scale: f32,
}

/// A listing's band: the listing colour, or the selected listing's orange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    Listing,
    Selected,
}

impl Band {
    /// Its colour at the list's brightness.
    fn rgb(self, scale: f32) -> [f32; 3] {
        match self {
            Band::Listing => BAND_RGB,
            Band::Selected => SELECTED_RGB,
        }
        .map(|c| c * scale)
    }

    /// How far off its band-to-black line a pixel of its text may stray.
    fn foreign(self) -> f32 {
        match self {
            Band::Listing => FOREIGN,
            Band::Selected => SELECTED_FOREIGN,
        }
    }

    /// Whether `p` is the light rule between its listing's lines, at the list's brightness.
    fn rule(self, p: [u8; 3], scale: f32) -> bool {
        let rule = match self {
            Band::Listing => RULE_RGB,
            Band::Selected => SELECTED_RULE_RGB,
        };
        let (along, off) = along_and_off(p, rule.map(|c| c * scale));
        off <= RULE_TOL && along <= RULE_BRIGHTEST
    }

    /// The columns of the list that hold its band, clear of its edges.
    fn columns(self, list: &List, width: u32) -> (u32, u32) {
        let inset = match self {
            Band::Listing => EDGE,
            Band::Selected => SELECTED_INSET,
        };
        (list.x0 + inset, (list.x0 + BAND_W - EDGE).min(width))
    }
}

/// Which band `p` is the colour of, at the list's brightness `scale`.
fn band_of(p: [u8; 3], scale: f32) -> Option<Band> {
    if band_scale(p).is_some() {
        return Some(Band::Listing);
    }
    let orange = SELECTED_RGB.map(|c| c * scale);
    (0..3).all(|i| (p[i] as f32 - orange[i]).abs() <= SELECTED_TOL).then_some(Band::Selected)
}

/// The band's brightness next to the screenshots' colour, if `p` is its colour.
fn band_scale(p: [u8; 3]) -> Option<f32> {
    let k = p[2] as f32 / BAND_RGB[2];
    let close = BAND_SCALE.contains(&k) && (0..2).all(|i| (p[i] as f32 - k * BAND_RGB[i]).abs() <= BAND_TOL);
    close.then_some(k)
}

/// The item list: the column where the most band rows start, if enough do.
fn find_list(img: &RgbImage) -> Option<List> {
    let (w, h) = img.dimensions();
    let mut starts: HashMap<u32, Vec<(u32, f32)>> = HashMap::new();
    for y in 0..h {
        let mut x = 0;
        while x < w {
            let Some(k) = band_scale(img.get_pixel(x, y).0) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < w && band_scale(img.get_pixel(x, y).0).is_some() {
                x += 1;
            }
            if (x - start).abs_diff(BAND_W) <= BAND_SLACK {
                starts.entry(start).or_default().push((y, k));
            }
        }
    }
    // A run a pixel short at one end starts a column over: count each column with its
    // neighbours.
    let near = |x0: u32| (x0.saturating_sub(1)..=x0 + 1).filter_map(|x| starts.get(&x)).map(Vec::len).sum::<usize>();
    let x0 = *starts.keys().max_by_key(|&&x0| (near(x0), starts[&x0].len()))?;
    if near(x0) < MIN_BAND_ROWS {
        return None;
    }
    let mut rows: Vec<(u32, f32)> =
        (x0.saturating_sub(1)..=x0 + 1).filter_map(|x| starts.get(&x)).flatten().copied().collect();
    rows.sort_by_key(|row| row.0);
    let mut scales: Vec<f32> = rows.iter().map(|&(_, k)| k).collect();
    scales.sort_by(f32::total_cmp);
    let scale = scales[scales.len() / 2];
    Some(List { x0, y0: rows.first()?.0, y1: rows.last()?.0 + 1, scale })
}

/// A line of text in a listing: its inked columns and rows (ends exclusive), and the band
/// it is on.
#[derive(Debug, Clone, Copy)]
struct TextLine {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    band: Band,
}

/// A listing: its name line and price line, and whether nothing covers them.
struct Listing {
    name: TextLine,
    price: TextLine,
    clean: bool,
}

impl Listing {
    /// Its band's top: a set distance above its price line's.
    fn top(&self) -> u32 {
        self.price.y0.saturating_sub(BAND_ABOVE_PRICE)
    }
}

/// How much of the way from `band` (its colour at the list's brightness) to black `p` is,
/// and how far off that line it lies.
fn ink_of(p: [u8; 3], band: [f32; 3]) -> (f32, f32) {
    let (along, off) = along_and_off(p, band);
    ((1.0 - along).clamp(0.0, 1.0), off)
}

/// Where `p` lies along the line from black (0) to `colour` (1), and how far off it.
fn along_and_off(p: [u8; 3], colour: [f32; 3]) -> (f32, f32) {
    let norm = colour.iter().map(|c| c * c).sum::<f32>();
    let p = p.map(f32::from);
    let along = p.iter().zip(&colour).map(|(a, b)| a * b).sum::<f32>() / norm;
    let off = p.iter().zip(&colour).map(|(a, b)| (a - along * b).powi(2)).sum::<f32>().sqrt();
    (along, off)
}

/// The listings in `list`, top to bottom: pairs of text lines, the name and the price
/// [`PRICE_BELOW`] px under it, on scanlines that are mostly band. Sold-out listings and
/// empty slots have none.
fn listings(img: &RgbImage, list: &List) -> Vec<Listing> {
    let (w, h) = img.dimensions();
    let (x0, x1) = (list.x0 + EDGE, (list.x0 + BAND_W - EDGE).min(w));
    // A selected listing above or below the bands found is read too.
    let reach = BAND_H + PRICE_BELOW;
    let (y0, y1) = (list.y0.saturating_sub(reach), (list.y1 + reach).min(h));
    // The band a scanline is text on: inked, and mostly band of one colour.
    let text_row = |y: u32| {
        let (mut listing, mut selected) = (0, 0);
        for x in x0..x1 {
            match band_of(img.get_pixel(x, y).0, list.scale) {
                Some(Band::Listing) => listing += 1,
                Some(Band::Selected) => selected += 1,
                None => {}
            }
        }
        let band = if listing >= selected { Band::Listing } else { Band::Selected };
        let (bx0, bx1) = band.columns(list, w);
        if listing.max(selected) as f32 <= TEXT_ROW_BAND * (bx1 - bx0) as f32 {
            return None;
        }
        let rgb = band.rgb(list.scale);
        let inked = (bx0..bx1).any(|x| {
            let (ink, off) = ink_of(img.get_pixel(x, y).0, rgb);
            ink >= INK_MIN && off <= band.foreign()
        });
        inked.then_some(band)
    };
    let rows: Vec<Option<Band>> = (y0..y1).map(text_row).collect();
    let mut lines = Vec::new();
    let mut y = 0;
    while y < rows.len() {
        let Some(band) = rows[y] else {
            y += 1;
            continue;
        };
        let top = y;
        while y < rows.len() && rows[y] == Some(band) {
            y += 1;
        }
        lines.push(line_extent(img, list, band, y0 + top as u32, y0 + y as u32));
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let below = lines.get(i + 1).filter(|next| next.band == lines[i].band).map(|next| next.y0 - lines[i].y0);
        match below {
            Some(below) if below.abs_diff(PRICE_BELOW) <= PRICE_SLACK => {
                let (name, price) = (lines[i], lines[i + 1]);
                let clean = clean(img, list, &name) && clean(img, list, &price);
                out.push(Listing { name, price, clean });
                i += 2;
            }
            // A line on its own: the other is covered, or it isn't a listing's.
            _ => i += 1,
        }
    }
    out
}

/// The text line on scanlines `y0..y1` of the list, on `band`: its inked columns.
fn line_extent(img: &RgbImage, list: &List, band: Band, y0: u32, y1: u32) -> TextLine {
    let (x0, x1) = band.columns(list, img.width());
    let rgb = band.rgb(list.scale);
    let inked = |x: u32| (y0..y1).any(|y| ink_of(img.get_pixel(x, y).0, rgb).0 >= INK_MIN);
    let first = (x0..x1).find(|&x| inked(x)).unwrap_or(x0);
    let last = (x0..x1).rev().find(|&x| inked(x)).map_or(x1, |x| x + 1);
    TextLine { x0: first, x1: last.max(first + 1), y0, y1, band }
}

/// Whether every pixel of the band across `line`'s rows, and a row above and below, is the
/// band, its text or the rule under the name: nothing covers it.
fn clean(img: &RgbImage, list: &List, line: &TextLine) -> bool {
    let band = line.band;
    let (x0, x1) = band.columns(list, img.width());
    let (y0, y1) = (line.y0.saturating_sub(1), (line.y1 + 1).min(img.height()));
    let rgb = band.rgb(list.scale);
    let shops = |p: [u8; 3]| ink_of(p, rgb).1 <= band.foreign() || band.rule(p, list.scale);
    (y0..y1).all(|y| (x0..x1).all(|x| shops(img.get_pixel(x, y).0)))
}

/// Whether the slot whose band starts at `top` holds pixels the shop never draws: neither
/// band nor its text, nor a sold-out listing's light greys, nor the light rule between a
/// listing's lines. Its listing, if it has one, is covered.
fn covered(img: &RgbImage, list: &List, top: u32) -> bool {
    let (w, h) = img.dimensions();
    let (x0, x1) = (list.x0 + EDGE, (list.x0 + BAND_W - EDGE).min(w));
    let (listing, selected) = (Band::Listing.rgb(list.scale), Band::Selected.rgb(list.scale));
    let shops = |p: [u8; 3]| {
        let light_grey = {
            let (lo, hi) = (*p.iter().min().unwrap_or(&0), *p.iter().max().unwrap_or(&0));
            hi - lo <= 8 && (150.0 * list.scale..=235.0 * list.scale).contains(&f32::from(hi))
        };
        light_grey
            || ink_of(p, listing).1 <= FOREIGN
            || ink_of(p, selected).1 <= SELECTED_FOREIGN
            || Band::Selected.rule(p, list.scale)
    };
    let foreign = (top..(top + BAND_H).min(h)).flat_map(|y| (x0..x1).map(move |x| (x, y)));
    foreign.filter(|&(x, y)| !shops(img.get_pixel(x, y).0)).nth(COVERED_MIN - 1).is_some()
}

/// `line`'s ink, black on white.
fn line_ink(img: &RgbImage, line: &TextLine, scale: f32) -> GrayImage {
    let (w, h) = (line.x1 - line.x0, line.y1 - line.y0);
    let rgb = line.band.rgb(scale);
    GrayImage::from_fn(w, h, |x, y| {
        let (ink, _) = ink_of(img.get_pixel(line.x0 + x, line.y0 + y).0, rgb);
        Luma([(255.0 * (1.0 - ink)).round() as u8])
    })
}

/// A price line as the game writes it: "<quantity> for <mesos> mesos", the mesos grouped in
/// threes with commas ("1 for 12,999 mesos"). `None` for anything else.
pub(crate) fn parse_price(read: &str) -> Option<ListedPrice> {
    let words: Vec<&str> = read.split_whitespace().collect();
    let [quantity, "for", mesos, "mesos"] = words[..] else { return None };
    let quantity: u64 = digits(quantity)?;
    let mesos = grouped(mesos)?;
    (quantity > 0).then_some(ListedPrice { quantity, mesos })
}

fn digits(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) && !text.starts_with('0')).then(|| text.parse().ok())?
}

/// A number with commas where the game puts them: 1 to 3 digits, then groups of 3.
fn grouped(text: &str) -> Option<u64> {
    let mut groups = text.split(',');
    let first = groups.next()?;
    if first.is_empty() || first.len() > 3 {
        return None;
    }
    let rest: Vec<&str> = groups.collect();
    if rest.iter().any(|group| group.len() != 3 || !group.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let all: String = std::iter::once(first).chain(rest).collect();
    digits(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_read_only_in_the_games_shape() {
        assert_eq!(parse_price("1 for 999 mesos"), Some(ListedPrice { quantity: 1, mesos: 999 }));
        assert_eq!(parse_price("10 for 12,999 mesos"), Some(ListedPrice { quantity: 10, mesos: 12_999 }));
        assert_eq!(parse_price("1 for 1,234,567 mesos"), Some(ListedPrice { quantity: 1, mesos: 1_234_567 }));
        for wrong in ["1 for 12999 mesos", "1 for 1,29 mesos", "1 for 12,9999 mesos", "1for 999 mesos", "0 for 9 mesos", "1 for 999", "1 tor 999 mesos"] {
            assert_eq!(parse_price(wrong), None, "{wrong}");
        }
    }
}
