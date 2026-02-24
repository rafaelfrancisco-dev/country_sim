const SYLLABLES: &[&str] = &[
    "al", "an", "ar", "bel", "cor", "da", "del", "en", "el", "far", "gan", "hal", "ian", "jor",
    "ka", "lin", "mor", "nar", "or", "pa", "qua", "ran", "sel", "tor", "ul", "val", "wen", "yor",
    "zen", "bria", "cai", "dri", "evo", "fel", "gri", "hya", "iva", "jen", "kyl", "lora", "myr",
    "nor", "ova", "pyr", "quil", "ria", "syl", "tov", "uri", "vora", "wyn", "xan", "yva", "zor",
];

fn hash64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn generate_word(seed: u64, min_syllables: usize, max_syllables: usize) -> String {
    let syllable_span = max_syllables.saturating_sub(min_syllables) + 1;
    let syllable_count = min_syllables + (hash64(seed) as usize % syllable_span);

    let mut word = String::new();
    for idx in 0..syllable_count {
        let hashed = hash64(seed.wrapping_add((idx as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)));
        let syllable = SYLLABLES[hashed as usize % SYLLABLES.len()];
        word.push_str(syllable);
    }

    capitalize_ascii(&word)
}

fn capitalize_ascii(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    let mut result = String::new();
    result.push(first.to_ascii_uppercase());
    result.extend(chars);
    result
}

pub fn country_name_from_seed(seed: u64) -> String {
    generate_word(seed ^ 0xaca0_31b9, 2, 4)
}

pub fn person_name_from_seed(seed: u64) -> String {
    let first = generate_word(seed ^ 0xf00d_1234, 2, 3);
    let last = generate_word(seed ^ 0xbadc_0ffe, 2, 4);
    format!("{first} {last}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn country_name_is_single_token() {
        let name = country_name_from_seed(123);
        assert_eq!(name.split_whitespace().count(), 1);
    }

    #[test]
    fn person_name_has_two_parts() {
        let name = person_name_from_seed(456);
        assert_eq!(name.split_whitespace().count(), 2);
    }

    #[test]
    fn name_generation_is_deterministic() {
        let first = person_name_from_seed(777);
        let second = person_name_from_seed(777);
        assert_eq!(first, second);
    }
}
