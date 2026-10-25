//! Fuzzy matching simple, sin dependencias: subsecuencia con puntuación por contigüidad.

/// Puntúa `needle` contra `haystack` (None si no es subsecuencia).
/// Mayor = mejor. Prefiere coincidencias al inicio y consecutivas.
pub fn fuzzy_score(needle: &str, haystack: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let needle = needle.to_lowercase();
    let hay = haystack.to_lowercase();
    let hay: Vec<char> = hay.chars().collect();
    let mut idx: Vec<usize> = Vec::with_capacity(needle.len());
    let mut search_from = 0usize;
    for n in needle.chars() {
        let mut found = None;
        for (i, c) in hay.iter().enumerate().skip(search_from) {
            if *c == n {
                found = Some(i);
                break;
            }
        }
        let f = found?;
        idx.push(f);
        search_from = f + 1;
    }
    let mut score: i32 = 100;
    // Bonus por contigüidad.
    for pair in idx.windows(2) {
        if pair[1] == pair[0] + 1 {
            score += 8;
        }
    }
    // Bonus por coincidencia temprana.
    if let Some(&first) = idx.first() {
        score -= (first.min(40) as i32) / 2;
    }
    // Malus por caracteres tapándose en el medio.
    if let (Some(&l), Some(&l2)) = (idx.first(), idx.last()) {
        let span = (l2 - l + 1) as i32;
        score -= span - idx.len() as i32;
    }
    Some(score)
}

/// Filtra y ordena `(valor, texto_de_búsqueda)` por score descendente.
pub fn filter_ranked<T: Clone>(
    query: &str,
    items: &[(T, String)],
) -> Vec<(i32, T)> {
    let mut scored: Vec<(i32, T)> = items
        .iter()
        .filter_map(|(v, text)| fuzzy_score(query, text).map(|s| (s, v.clone())))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored
}
