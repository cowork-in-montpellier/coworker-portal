/// Strips French accents and uppercases, mirroring the source site's own
/// word normalization (`nettoyerMot`) so stored words match its dictionaries.
pub fn clean_word(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(strip_diacritic)
        .collect()
}

fn strip_diacritic(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'À' | 'Â' | 'Ä' | 'Á' | 'Ã' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'î' | 'ï' | 'í' | 'ì' | 'Î' | 'Ï' | 'Í' | 'Ì' => 'I',
        'ô' | 'ö' | 'ó' | 'ò' | 'Ô' | 'Ö' | 'Ó' | 'Ò' => 'O',
        'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' | 'Ú' => 'U',
        'ç' | 'Ç' => 'C',
        'ÿ' | 'Ÿ' => 'Y',
        'ñ' | 'Ñ' => 'N',
        'œ' | 'Œ' => 'O',
        'æ' | 'Æ' => 'A',
        other => other.to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_accents_and_uppercases() {
        assert_eq!(clean_word("blouson"), "BLOUSON");
        assert_eq!(clean_word("Écœuré "), "ECOURE");
    }
}
