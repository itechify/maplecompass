//! The Personal Bests page in the player's browser: the core's [`BestsPage`] filled into
//! `personal-bests.html`, written afresh beside the bests file
//! (`%APPDATA%\com.itechify.maplecompass\personal-bests.html` on Windows) each time the
//! player opens it, and opened with whatever opens their web pages. The page is one file,
//! with the shared theme, the fonts, the Grind Tracker's icon and the page's figures all
//! inline: a page opened from a file can't load fonts from other files (Chromium treats
//! each as another origin).

use crate::files;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use compass_core::BestsPage;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

/// The page, with `/*@THEME@*/`, `@ICON@` and `@PAGE@` to fill in.
const TEMPLATE: &str = include_str!("personal-bests.html");
/// The shared window language, as every window has it.
const THEME: &str = include_str!("../../src/theme.css");
/// The theme's fonts, which it names by these paths, relative to `src/`.
const FONTS: [(&str, &[u8]); 2] = [
    ("../assets/fonts/Maplestory-Light.ttf", include_bytes!("../../assets/fonts/Maplestory-Light.ttf")),
    ("../assets/fonts/Maplestory-Bold.ttf", include_bytes!("../../assets/fonts/Maplestory-Bold.ttf")),
];
const ICON: &[u8] = include_bytes!("../../assets/grind-tracker.png");

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("personal-bests.html"))
}

/// Write the page and open it in the player's browser. Failing either is only logged:
/// the player can press the button again.
pub fn open(app: &AppHandle, page: &BestsPage) {
    let path = match path(app) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("no folder for the Personal Bests page: {e}");
            return;
        }
    };
    if let Err(e) = files::write_atomically(&path, render(page).as_bytes()) {
        eprintln!("couldn't write {}: {e}", path.display());
    } else if let Err(e) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        eprintln!("couldn't open {}: {e}", path.display());
    }
}

/// The page for `page`: the template with the theme (its fonts inline) and the icon (the
/// tab's and the window bar's), made once, then the page's figures as JSON.
fn render(page: &BestsPage) -> String {
    static FILLED: OnceLock<String> = OnceLock::new();
    let filled = FILLED.get_or_init(|| {
        let theme = FONTS.iter().fold(THEME.to_string(), |theme, (path, font)| {
            theme.replace(path, &format!("data:font/ttf;base64,{}", STANDARD.encode(font)))
        });
        let icon = format!("data:image/png;base64,{}", STANDARD.encode(ICON));
        TEMPLATE.replacen("/*@THEME@*/", &theme, 1).replace("@ICON@", &icon)
    });
    // In a script element, `</script>` in a map's name would end it: no `<` is left as is
    // (JSON's \u escape for it is valid wherever `<` can be, in strings).
    let json = serde_json::to_string(page).expect("the page is plain data").replace('<', "\\u003c");
    filled.replacen("@PAGE@", &json, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use compass_core::{BestsPageMap, LevelRange, OSMS_CREDIT};

    /// A page of one map, named `map_name`, with made-up figures.
    fn page_of(map_name: &str) -> BestsPage {
        BestsPage {
            as_of_unix_ms: Some(1_791_547_200_000),
            maps: vec![BestsPageMap {
                id: "010003070".into(),
                street_name: "Victoria Road".into(),
                map_name: map_name.into(),
                region: "Victoria Island".into(),
                url: "https://meowdb.com/msclassic/maps/010003070".into(),
                mob_levels: Some(LevelRange { lowest: 2, highest: 14 }),
                exp_per_hour: 7200,
                solo_exp_ceiling: Some(213_571),
                ceiling_share_hundredths: Some(337),
                set_at_unix_ms: None,
                here: true,
            }],
            credit: OSMS_CREDIT,
        }
    }

    /// The JSON the page reads its figures from.
    fn figures(html: &str) -> serde_json::Value {
        const OPEN: &str = r#"<script id="page" type="application/json">"#;
        let start = html.find(OPEN).expect("the page's figures") + OPEN.len();
        let end = start + html[start..].find("</script>").expect("their end");
        serde_json::from_str(&html[start..end]).expect("JSON")
    }

    #[test]
    fn the_page_carries_its_figures_as_the_core_gives_them() {
        let page = page_of("Sunset Sky");
        let html = render(&page);
        assert_eq!(figures(&html), serde_json::to_value(&page).unwrap());
        assert!(!html.contains("@PAGE@") && !html.contains("@ICON@") && !html.contains("@THEME@"));
    }

    #[test]
    fn a_name_that_would_end_the_script_reads_as_it_is() {
        let page = page_of("Physical Fitness Test <Normal Waiting Room></script><b>");
        let html = render(&page);
        assert_eq!(html.matches("</script>").count(), render(&page_of("Sunset Sky")).matches("</script>").count());
        assert_eq!(figures(&html)["maps"][0]["mapName"], "Physical Fitness Test <Normal Waiting Room></script><b>");
    }

    #[test]
    fn the_fonts_and_icon_are_inline_so_the_page_needs_no_other_file() {
        let html = render(&page_of("Sunset Sky"));
        assert!(!html.contains("../assets/"), "a font still named by its path in theme.css");
        assert_eq!(html.matches("data:font/ttf;base64,").count(), 2);
        // In the window bar, and as the tab's icon.
        assert!(html.contains(r#"src="data:image/png;base64,"#));
        assert!(html.contains(r#"rel="icon" type="image/png" href="data:image/png;base64,"#));
    }
}
