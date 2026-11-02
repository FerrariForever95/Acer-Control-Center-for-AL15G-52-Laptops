/// Words that stay upper-case when a shouting label is softened.
const ACRONYMS: [&str; 13] = [
    "CPU", "GPU", "RGB", "RPM", "USB", "LED", "PCI", "NVIDIA", "BIOS", "AC",
    "DC", "RTD3", "EC",
];

/// Turn an ALL-CAPS label into sentence case ("LIVE COOLING" becomes
/// "Live cooling"), keeping acronyms such as CPU and GPU upper-case.
///
/// Text that already contains lower-case letters, or has no letters at all
/// ("--", "45 %"), is returned unchanged.
pub fn sentence_case(text: &str) -> String {
    let has_letters = text.chars().any(|c| c.is_alphabetic());
    let has_lower = text.chars().any(|c| c.is_lowercase());

    if !has_letters || has_lower {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut first_word = true;

    for (i, word) in text.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }

        if ACRONYMS.contains(&word) {
            out.push_str(word);
            first_word = false;
            continue;
        }

        // Tokens with digits or symbols ("°C", "RTD3", "45%", "N/A") are
        // not prose; leave them exactly as written.
        if word.chars().any(|c| !c.is_alphabetic()) {
            out.push_str(word);

            if word.chars().any(|c| c.is_alphabetic()) {
                first_word = false;
            }

            continue;
        }

        let lower = word.to_lowercase();
        let mut chars = lower.chars();

        if first_word {
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
            }
        }

        out.push_str(chars.as_str());
        first_word = false;
    }

    out
}
