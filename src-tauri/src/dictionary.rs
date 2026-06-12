use crate::config::DictEntry;

/// Comma-joined vocabulary for whisper's --prompt bias, built from the
/// "to" side of every entry. None when the dictionary is empty.
pub fn vocab_prompt(entries: &[DictEntry]) -> Option<String> {
    let words: Vec<&str> = entries
        .iter()
        .map(|e| e.to.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if words.is_empty() {
        None
    } else {
        Some(words.join(", "))
    }
}

/// Apply replacement pairs: whole-word, case-insensitive.
/// Entries with an empty "from" are vocabulary hints only and skipped here.
pub fn apply(text: &str, entries: &[DictEntry]) -> String {
    let mut out = text.to_string();
    for e in entries {
        let from = e.from.trim();
        let to = e.to.trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        out = replace_word_ci(&out, from, to);
    }
    out
}

/// Compare the original transcription with the user-edited version and
/// extract replacement pairs. Conservative: only when both texts have the
/// same word count (positional diff), at most 3 substitutions, and the
/// change is more than just letter case.
pub fn learn(original: &str, edited: &str) -> Vec<DictEntry> {
    let o = words(original);
    let e = words(edited);
    if o.is_empty() || o.len() != e.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (a, b) in o.iter().zip(e.iter()) {
        if a == b || a.eq_ignore_ascii_case(b) {
            continue;
        }
        out.push(DictEntry { from: a.to_lowercase(), to: b.clone() });
        if out.len() > 3 {
            return Vec::new();
        }
    }
    out
}

/// Insert or update an entry; returns true when the dictionary changed.
pub fn merge_entry(dict: &mut Vec<DictEntry>, e: DictEntry) -> bool {
    if let Some(existing) = dict
        .iter_mut()
        .find(|d| !d.from.is_empty() && d.from.eq_ignore_ascii_case(&e.from))
    {
        if existing.to == e.to {
            return false;
        }
        existing.to = e.to;
        true
    } else {
        dict.push(e);
        true
    }
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect()
}

fn replace_word_ci(text: &str, from: &str, to: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let pat: Vec<char> = from.chars().flat_map(|c| c.to_lowercase()).collect();
    let n = chars.len();
    let w = from.chars().count();
    if w == 0 || w > n {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        let is_match = i + w <= n
            && chars[i..i + w]
                .iter()
                .flat_map(|c| c.to_lowercase())
                .eq(pat.iter().copied())
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && (i + w == n || !chars[i + w].is_alphanumeric());
        if is_match {
            out.push_str(to);
            i += w;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(from: &str, to: &str) -> DictEntry {
        DictEntry { from: from.into(), to: to.into() }
    }

    #[test]
    fn replaces_whole_word_case_insensitive() {
        let d = [entry("cloud", "Claude")];
        assert_eq!(apply("I asked cloud about it.", &d), "I asked Claude about it.");
        assert_eq!(apply("Cloud said yes.", &d), "Claude said yes.");
    }

    #[test]
    fn does_not_touch_substrings() {
        let d = [entry("cloud", "Claude")];
        assert_eq!(apply("cloudy clouds in the cloud", &d), "cloudy clouds in the Claude");
    }

    #[test]
    fn multi_word_from() {
        let d = [entry("cursor a i", "Cursor AI")];
        assert_eq!(apply("open cursor a i now", &d), "open Cursor AI now");
    }

    #[test]
    fn learn_single_substitution() {
        let got = learn("I asked cloud about it.", "I asked Claude about it.");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].from, "cloud");
        assert_eq!(got[0].to, "Claude");
    }

    #[test]
    fn learn_ignores_case_only_and_rewrites() {
        assert!(learn("Hello world.", "hello world.").is_empty());
        assert!(learn("one two three", "totally different text here now").is_empty());
    }

    #[test]
    fn merge_updates_existing() {
        let mut d = vec![entry("cloud", "Cloud9")];
        assert!(merge_entry(&mut d, entry("cloud", "Claude")));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].to, "Claude");
        assert!(!merge_entry(&mut d, entry("Cloud", "Claude")));
    }

    #[test]
    fn empty_from_is_bias_only() {
        let d = [entry("", "Tauri")];
        assert_eq!(apply("tauri app", &d), "tauri app");
        assert_eq!(vocab_prompt(&d).as_deref(), Some("Tauri"));
    }
}
