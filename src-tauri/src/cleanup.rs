pub fn basic_cleanup(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len() + 4);
    let mut capitalize_next = true;
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
            continue;
        }
        prev_space = false;
        if capitalize_next && ch.is_alphabetic() {
            for u in ch.to_uppercase() {
                out.push(u);
            }
            capitalize_next = false;
        } else {
            out.push(ch);
        }
        if matches!(ch, '.' | '!' | '?') {
            capitalize_next = true;
        }
    }
    let needs_period = !matches!(out.chars().last(), Some('.') | Some('!') | Some('?') | Some(','));
    if needs_period && !out.is_empty() {
        out.push('.');
    }
    out
}
