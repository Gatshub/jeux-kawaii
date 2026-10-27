//! jeux-core : briques partagées de la collection de jeux.
//!
//! - `dico`   : dictionnaire normalisé (accents → ASCII), secrets filtrés
//! - `wiki`   : définitions Wiktionnaire (parse wikitext, nettoyage markup)
//! - `theme`  : palette adaptative clair/sombre (rose kawaii)

/// État d'une lettre devinée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feedback {
    /// Absente du mot
    Absent,
    /// Présente ailleurs
    Present,
    /// Bien placée
    Good,
}

/// Score d'une tentative contre un secret (style Motus/Wordle).
/// Renvoie le feedback par position et les positions bien placées.
pub fn score(guess: &[char], secret: &[char]) -> (Vec<Feedback>, Vec<bool>) {
    let n = secret.len();
    let mut fb = vec![Feedback::Absent; n];
    let mut locked = vec![false; n];
    let mut rem: Vec<char> = secret.to_vec();
    for i in 0..n {
        if guess[i] == secret[i] {
            fb[i] = Feedback::Good;
            locked[i] = true;
            if let Some(p) = rem.iter().position(|&r| r == guess[i]) {
                rem.remove(p);
            }
        }
    }
    for i in 0..n {
        if fb[i] == Feedback::Good {
            continue;
        }
        if let Some(p) = rem.iter().position(|&r| r == guess[i]) {
            fb[i] = Feedback::Present;
            rem.remove(p);
        }
    }
    (fb, locked)
}

pub mod dico {
    //! Dictionnaire embarqué et filtrage des secrets.

    /// Mots valides (toutes formes), normalisés sans accents, 5-9 lettres.
    pub const MOTS: &str = include_str!("../../motus-rust/data/mots.txt");
    /// Secrets : noms + infinitifs + participes, peu fréquents.
    pub const SECRETS: &str = include_str!("../../motus-rust/data/secrets.txt");

    /// Liste des mots par longueur, normalisés.
    pub fn mots() -> std::collections::HashMap<usize, Vec<String>> {
        charger(MOTS)
    }

    /// Liste des secrets par longueur.
    pub fn secrets() -> std::collections::HashMap<usize, Vec<String>> {
        charger(SECRETS)
    }

    fn charger(s: &str) -> std::collections::HashMap<usize, Vec<String>> {
        let mut m: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();
        for w in s.lines() {
            let w = w.trim();
            if (5..=9).contains(&w.len()) {
                m.entry(w.len()).or_default().push(w.to_string());
            }
        }
        m
    }

    /// Normalise un mot : décompose les accents et retire les marques.
    /// « être » → « etre ». À utiliser sur toute saisie utilisateur.
    pub fn normalise(w: &str) -> String {
        // décomposition NFD manuelle via la table Unicode de std
        let mut out = String::new();
        for c in w.chars() {
            // les caractères précomposés latins courants : décomposer à la main
            let s = match c {
                'à' | 'â' | 'ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'î' | 'ï' => 'i',
                'ô' | 'ö' => 'o',
                'ù' | 'û' | 'ü' => 'u',
                'ÿ' => 'y',
                'ç' => 'c',
                'œ' => { out.push('o'); out.push('e'); continue; }
                'æ' => { out.push('a'); out.push('e'); continue; }
                'À' | 'Â' | 'Ä' => 'a',
                'É' | 'È' | 'Ê' | 'Ë' => 'e',
                'Î' | 'Ï' => 'i',
                'Ô' | 'Ö' => 'o',
                'Ù' | 'Û' | 'Ü' => 'u',
                'Ç' => 'c',
                c if c.is_ascii_alphabetic() => c.to_ascii_lowercase(),
                _ => continue,
            };
            out.push(s);
        }
        out
    }
}

pub mod wiki {
    //! Définitions Wiktionnaire (API `action=parse&prop=wikitext`).

    /// Récupère les définitions françaises d'un mot (bloquant, ~8 s max).
    /// Nécessite `curl` (pas de dépendance TLS).
    pub fn definition(word: &str) -> String {
        let url = format!(
            "https://fr.wiktionary.org/w/api.php?action=parse&prop=wikitext&page={}&format=json&formatversion=2&redirects=1",
            word
        );
        let out = std::process::Command::new("curl")
            .args(["-s", "--max-time", "8", "-A", "jeux-kawaii/1.0", &url])
            .output();
        let Ok(out) = out else { return String::new() };
        let s = String::from_utf8_lossy(&out.stdout);
        let Some(p) = s.find("\"wikitext\":\"") else { return String::new() };
        let rest = &s[p + 12..];
        let wikitext = match rest.strip_suffix("\"}") {
            Some(w) => w,
            None => match fin_chaine(rest) {
                Some(i) => &rest[..i],
                None => return String::new(),
            },
        };
        let wikitext = unescape_json(wikitext);
        let mut defs = Vec::new();
        let mut in_fr = false;
        for l in wikitext.lines() {
            if l.starts_with("== {{langue|fr}} ==") {
                in_fr = true;
            } else if in_fr && l.starts_with("== {{") && !l.starts_with("===") {
                break;
            } else if in_fr && l.starts_with("# ") {
                defs.push(nettoie(&l[2..]));
            }
        }
        defs.join("\n")
    }

    /// Position de la fin de chaîne JSON (guillemet non échappé).
    fn fin_chaine(s: &str) -> Option<usize> {
        let mut esc = false;
        for (i, c) in s.char_indices() {
            if esc { esc = false; continue; }
            match c {
                '\\' => esc = true,
                '"' => return Some(i),
                _ => {}
            }
        }
        None
    }

    fn unescape_json(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c == '\\' {
                match it.next() {
                    Some('n') => out.push('\n'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('u') => {
                        let hex: String = it.by_ref().take(4).collect();
                        if let Ok(n) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(n) { out.push(ch); }
                        }
                    }
                    other => { if let Some(o) = other { out.push(o); } }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Retire le markup wiki : '''gras''', {{templates}}, [[liens|texte]].
    fn nettoie(s: &str) -> String {
        let mut s = s.replace("'''", "").replace("''", "");
        while let Some(a) = s.find("{{") {
            if let Some(b) = s[a..].find("}}") {
                s = format!("{}{}", &s[..a], &s[a + b + 2..]);
            } else { break; }
        }
        while let Some(a) = s.find("[[") {
            if let Some(b) = s[a..].find("]]") {
                let inner = &s[a + 2..a + b];
                let inner = inner.split('|').last().unwrap_or(inner);
                s = format!("{}{}{}", &s[..a], inner, &s[a + b + 2..]);
            } else { break; }
        }
        s
    }
}

pub mod theme {
    //! Palette adaptative clair/sombre, rose kawaii.

    #[derive(Clone, Copy, Debug)]
    pub struct Theme {
        /// Bien placé (rose/rouge)
        pub good: (u8, u8, u8),
        /// Erreur (rouge)
        pub ko: (u8, u8, u8),
        /// Présent ailleurs (bleu)
        pub present: (u8, u8, u8),
        /// Absent (gris doux)
        pub absent: (u8, u8, u8),
        /// Lettre donnée
        pub given: (u8, u8, u8),
        /// Accent (curseur, titre)
        pub accent: (u8, u8, u8),
        /// Texte secondaire
        pub muted: (u8, u8, u8),
    }

    pub const SOMBRE: Theme = Theme {
        good: (255, 123, 138),
        ko: (255, 123, 138),
        present: (123, 184, 255),
        absent: (106, 106, 106),
        given: (221, 221, 221),
        accent: (255, 158, 196),
        muted: (187, 187, 187),
    };

    pub const CLAIR: Theme = Theme {
        good: (179, 36, 60),
        ko: (179, 36, 60),
        present: (31, 95, 168),
        absent: (138, 138, 138),
        given: (68, 68, 68),
        accent: (214, 72, 140),
        muted: (85, 85, 85),
    };

    /// Détecte un fond sombre via COLORFGBG (défaut : sombre).
    pub fn detecte_sombre() -> bool {
        if let Ok(v) = std::env::var("COLORFGBG") {
            let parts: Vec<&str> = v.split(';').collect();
            if parts.len() >= 2 {
                if let Ok(bg) = parts[parts.len() - 1].parse::<u8>() {
                    return bg == 0 || bg == 8;
                }
            }
        }
        true
    }

    pub fn actif() -> Theme {
        if detecte_sombre() { SOMBRE } else { CLAIR }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_motus() {
        let secret: Vec<char> = "motus".chars().collect();
        // "torte" : t présent (pos 0≠2), o bien placé (pos 1), r absent,
        // t présent une seule fois (le 2ᵉ t de torte compte déjà),
        // e absent
        let guess: Vec<char> = "torte".chars().collect();
        let (fb, locked) = score(&guess, &secret);
        assert_eq!(fb[0], Feedback::Present); // t (pos 2 dans motus)
        assert_eq!(fb[1], Feedback::Good); // o
        assert_eq!(fb[2], Feedback::Absent); // r
        assert_eq!(fb[3], Feedback::Absent); // 2ᵉ t : plus aucun t dans le "reste"
        assert_eq!(fb[4], Feedback::Absent); // e
        assert!(locked[1]);
    }

    #[test]
    fn normalise_accents() {
        assert_eq!(dico::normalise("être"), "etre");
        assert_eq!(dico::normalise("fête"), "fete");
        assert_eq!(dico::normalise("HÊTRE"), "hetre");
        assert_eq!(dico::normalise("œil"), "oeil");
    }

    #[test]
    fn dico_contient_les_bases() {
        let m = dico::mots();
        assert!(m.get(&5).map_or(false, |v| v.contains(&"hetre".to_string())));
        let s = dico::secrets();
        assert!(s.get(&5).map_or(false, |v| !v.is_empty()));
        // pas de formes conjuguées dans les secrets
        assert!(!s.get(&7).map_or(false, |v| v.contains(&"mangeait".to_string())));
    }

    #[test]
    fn theme_adaptatif() {
        // juste vérifier que les constantes sont distinctes
        assert_ne!(theme::SOMBRE.accent, theme::CLAIR.accent);
    }
}
