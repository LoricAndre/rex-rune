//! Fonts: the kit's bundled Inter and IBM Plex Mono for chrome, plus the user's terminal font
//! resolved through fontconfig and registered by its real family name.
use std::process::Command;

pub struct Fonts {
    pub files: &'static [&'static [u8]],
    pub families: (&'static str, &'static str),
    /// The family the terminal grid draws in, or `None` for the bundled mono.
    pub terminal: Option<&'static str>,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Every face fontconfig resolves for `family` in four styles, deduplicated, and the family
/// name the files declare. Returns nothing when fontconfig substitutes another family.
fn resolve(family: &str) -> Option<(String, Vec<Vec<u8>>)> {
    let mut name = None;
    let mut files = Vec::new();
    let mut seen = Vec::new();
    for style in ["", ":bold", ":italic", ":bold:italic"] {
        let out = Command::new("fc-match")
            .args(["-f", "%{family[0]}\n%{file}", &format!("{family}{style}")])
            .output()
            .ok()?;
        let out = String::from_utf8_lossy(&out.stdout).into_owned();
        let (found, file) = out.split_once('\n')?;
        if !found.eq_ignore_ascii_case(family) {
            return None;
        }
        name.get_or_insert_with(|| found.to_owned());
        if !seen.contains(&file.to_owned())
            && let Ok(bytes) = std::fs::read(file)
        {
            seen.push(file.to_owned());
            files.push(bytes);
        }
    }
    Some((name?, files)).filter(|(_, f)| !f.is_empty())
}

pub fn load(terminal_family: &str) -> Fonts {
    let mut files: Vec<&'static [u8]> = rune_theme_opencode::FONTS.to_vec();
    let terminal = match resolve(terminal_family) {
        Some((name, faces)) => {
            files.extend(faces.into_iter().map(|f| &*Box::leak(f.into_boxed_slice())));
            Some(leak(name))
        }
        None => {
            eprintln!("Terminal font {terminal_family:?} not found; using IBM Plex Mono.");
            None
        }
    };
    Fonts {
        files: Box::leak(files.into_boxed_slice()),
        families: rune_theme_opencode::FAMILIES,
        terminal,
    }
}
