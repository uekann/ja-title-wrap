use std::sync::OnceLock;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer as LinderaTokenizer;
use serde::Serialize;

const NO_BREAK_BEFORE: &[&str] = &[
    "、", "。", "）", "」", "』", "】", "》", "〉", "，", "．", "!", "?", "！", "？", "：", "；",
];
const NO_BREAK_AFTER: &[&str] = &["（", "「", "『", "【", "《", "〈"];
const PARTICLES: &[&str] = &[
    "は", "が", "を", "に", "へ", "で", "と", "や", "の", "も", "から", "まで", "より", "か", "ね",
    "よ", "って", "など",
];

#[derive(Debug, Clone, Serialize)]
struct Analysis<'a> {
    tokens: Vec<String>,
    break_after: Vec<usize>,
    no_break_before: &'a [&'a str],
    no_break_after: &'a [&'a str],
}

#[derive(Debug, Clone)]
struct TokenUnit {
    surface: String,
    pos_major: String,
}

static TOKENIZER: OnceLock<Result<LinderaTokenizer, String>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
wasm_minimal_protocol::initiate_protocol!();

#[cfg_attr(target_arch = "wasm32", wasm_minimal_protocol::wasm_func)]
pub fn analyze_ja_title(input: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(input).map_err(|err| err.to_string())?;
    let analysis = analyze_text(text)?;
    serde_json::to_vec(&analysis).map_err(|err| err.to_string())
}

fn analyze_text(text: &str) -> Result<Analysis<'static>, String> {
    let units = tokenize_units(text)?;
    let tokens = units.iter().map(|unit| unit.surface.clone()).collect();
    let break_after = collect_break_candidates(&units);
    Ok(Analysis {
        tokens,
        break_after,
        no_break_before: NO_BREAK_BEFORE,
        no_break_after: NO_BREAK_AFTER,
    })
}

#[cfg(test)]
fn tokenize(text: &str) -> Result<Vec<String>, String> {
    Ok(tokenize_units(text)?
        .into_iter()
        .map(|unit| unit.surface)
        .collect())
}

fn tokenize_units(text: &str) -> Result<Vec<TokenUnit>, String> {
    Ok(normalize_units(tokenize_with_lindera(text)?))
}

fn tokenize_with_lindera(text: &str) -> Result<Vec<TokenUnit>, String> {
    let tokenizer = tokenizer()?;
    let mut tokens = tokenizer.tokenize(text).map_err(|err| err.to_string())?;
    let mut units = Vec::with_capacity(tokens.len());
    for token in tokens.iter_mut() {
        let pos_major = token.get_detail(0).unwrap_or("").to_string();
        units.push(TokenUnit {
            surface: token.surface.as_ref().to_string(),
            pos_major,
        });
    }
    Ok(units)
}

fn tokenizer() -> Result<&'static LinderaTokenizer, String> {
    TOKENIZER
        .get_or_init(build_tokenizer)
        .as_ref()
        .map_err(Clone::clone)
}

fn build_tokenizer() -> Result<LinderaTokenizer, String> {
    let dictionary = load_dictionary("embedded://ipadic").map_err(|err| err.to_string())?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
    Ok(LinderaTokenizer::new(segmenter))
}

fn normalize_units(units: Vec<TokenUnit>) -> Vec<TokenUnit> {
    let mut normalized = Vec::with_capacity(units.len());
    for unit in units {
        if is_whitespace_token(&unit.surface) {
            push_single_space_unit(&mut normalized);
        } else if !unit.surface.is_empty() {
            normalized.push(unit);
        }
    }
    trim_edge_space_units(normalized)
}

fn is_whitespace_token(surface: &str) -> bool {
    !surface.is_empty() && surface.chars().all(char::is_whitespace)
}

fn push_single_space_unit(tokens: &mut Vec<TokenUnit>) {
    let needs_space = tokens.last().map(|t| t.surface.as_str()) != Some(" ");
    if needs_space {
        tokens.push(TokenUnit {
            surface: " ".to_string(),
            pos_major: "記号".to_string(),
        });
    }
}

fn trim_edge_space_units(mut units: Vec<TokenUnit>) -> Vec<TokenUnit> {
    while matches!(units.first().map(|t| t.surface.as_str()), Some(" ")) {
        units.remove(0);
    }
    while matches!(units.last().map(|t| t.surface.as_str()), Some(" ")) {
        units.pop();
    }
    units
}

fn collect_break_candidates(tokens: &[TokenUnit]) -> Vec<usize> {
    if tokens.len() <= 1 {
        return Vec::new();
    }

    let mut break_after = Vec::new();
    for i in 0..(tokens.len() - 1) {
        let left = &tokens[i];
        let right = &tokens[i + 1];
        if !can_break_between(left, right) {
            continue;
        }

        if is_boundary_strong(left, right) || i % 2 == 1 {
            break_after.push(i);
        }
    }

    if break_after.is_empty() {
        if let Some(i) = find_fallback_break(tokens) {
            break_after.push(i);
        }
    }

    break_after.sort_unstable();
    break_after.dedup();
    break_after
}

fn can_break_between(left: &TokenUnit, right: &TokenUnit) -> bool {
    if left.surface == " " || right.surface == " " {
        return false;
    }
    if is_particle(right) {
        return false;
    }
    if NO_BREAK_AFTER.contains(&left.surface.as_str())
        || NO_BREAK_BEFORE.contains(&right.surface.as_str())
    {
        return false;
    }
    true
}

fn find_fallback_break(tokens: &[TokenUnit]) -> Option<usize> {
    if tokens.len() <= 1 {
        return None;
    }
    let max = tokens.len() - 1;
    let center = max / 2;
    let mut candidates: Vec<usize> = (0..max).collect();
    candidates.sort_by_key(|&i| i.abs_diff(center));
    candidates
        .into_iter()
        .find(|&i| can_break_between(&tokens[i], &tokens[i + 1]))
}

fn is_boundary_strong(left: &TokenUnit, right: &TokenUnit) -> bool {
    is_particle(left)
        || left.pos_major == "記号"
        || NO_BREAK_BEFORE.contains(&left.surface.as_str())
        || right.surface.chars().count() >= 3
}

fn is_particle(token: &TokenUnit) -> bool {
    token.pos_major == "助詞" || PARTICLES.contains(&token.surface.as_str())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::Analysis;
    use super::TokenUnit;
    use super::analyze_ja_title;
    use super::analyze_text;
    use super::can_break_between;
    use super::collect_break_candidates;
    use super::find_fallback_break;
    use super::is_boundary_strong;
    use super::is_particle;
    use super::is_whitespace_token;
    use super::normalize_units;
    use super::push_single_space_unit;
    use super::tokenize;
    use super::tokenize_with_lindera;
    use super::trim_edge_space_units;

    #[test]
    fn plugin_returns_json() {
        let out = analyze_ja_title("長いタイトルを自然に改行する".as_bytes()).unwrap();
        let json: Value = serde_json::from_slice(&out).unwrap();
        assert!(json["tokens"].is_array());
        assert!(json["break_after"].is_array());
    }

    #[test]
    fn tokenizer_keeps_ascii_chunks() {
        let tokens = tokenize("Typst plugin を作る").unwrap();
        assert_eq!(tokens[0], "Typst");
        assert!(tokens.iter().any(|t| t == "plugin"));
    }

    #[test]
    fn tokenizer_splits_particle_as_token() {
        let tokens = tokenize("形態素解析ベースで長いタイトルを自然に改行する").unwrap();
        assert!(tokens.iter().any(|t| t == "を"));
    }

    #[test]
    fn break_candidates_do_not_split_before_particles() {
        let tokens = vec![
            token("自動", "名詞"),
            token("改行", "名詞"),
            token("を", "助詞"),
            token("実装", "名詞"),
            token("する", "動詞"),
        ];
        let breaks = collect_break_candidates(&tokens);
        assert!(!breaks.contains(&1)); // 改行|を を禁止
        assert!(breaks.contains(&2)); // を|実装 を許可
    }

    #[test]
    fn fallback_break_also_avoids_particle_front() {
        let tokens = vec![
            token("A", "名詞"),
            token("B", "名詞"),
            token("を", "助詞"),
            token("。", "記号"),
        ];
        let breaks = collect_break_candidates(&tokens);
        assert_eq!(breaks, vec![0]);
    }

    #[test]
    fn plugin_rejects_invalid_utf8() {
        let err = analyze_ja_title(&[0xff, 0xfe]).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn analyze_text_sets_kinsoku_metadata() {
        let analysis: Analysis = analyze_text("短いタイトル").unwrap();
        assert!(analysis.tokens.len() >= 2);
        assert!(analysis.no_break_before.contains(&"、"));
        assert!(analysis.no_break_after.contains(&"（"));
    }

    #[test]
    fn whitespace_detection_handles_spaces_and_tabs() {
        assert!(is_whitespace_token(" "));
        assert!(is_whitespace_token("\t\n"));
        assert!(!is_whitespace_token(" A "));
        assert!(!is_whitespace_token(""));
    }

    #[test]
    fn push_single_space_unit_deduplicates_spaces() {
        let mut tokens = vec![token("A", "名詞")];
        push_single_space_unit(&mut tokens);
        push_single_space_unit(&mut tokens);
        assert_eq!(surfaces(&tokens), vec!["A", " "]);
    }

    #[test]
    fn trim_edge_space_units_keeps_inner_spaces() {
        let units = vec![
            token(" ", "記号"),
            token("A", "名詞"),
            token(" ", "記号"),
            token("B", "名詞"),
            token(" ", "記号"),
        ];
        let trimmed = trim_edge_space_units(units);
        assert_eq!(surfaces(&trimmed), vec!["A", " ", "B"]);
    }

    #[test]
    fn normalize_units_cleans_empty_and_collapses_spaces() {
        let units = vec![
            token(" ", "記号"),
            token(" ", "記号"),
            token("A", "名詞"),
            token("", "名詞"),
            token("   ", "記号"),
            token("B", "名詞"),
            token("\n", "記号"),
        ];
        let normalized = normalize_units(units);
        assert_eq!(surfaces(&normalized), vec!["A", " ", "B"]);
    }

    #[test]
    fn can_break_between_respects_rules() {
        assert!(can_break_between(
            &token("自動", "名詞"),
            &token("改行", "名詞")
        ));
        assert!(!can_break_between(
            &token(" ", "記号"),
            &token("改行", "名詞")
        ));
        assert!(!can_break_between(
            &token("自動", "名詞"),
            &token("を", "助詞")
        ));
        assert!(!can_break_between(
            &token("（", "記号"),
            &token("改行", "名詞")
        ));
        assert!(!can_break_between(
            &token("自動", "名詞"),
            &token("。", "記号")
        ));
    }

    #[test]
    fn fallback_break_prefers_near_center_legal_boundary() {
        let tokens = vec![
            token("A", "名詞"),
            token("B", "名詞"),
            token("を", "助詞"),
            token("C", "名詞"),
            token("D", "名詞"),
        ];
        let idx = find_fallback_break(&tokens).unwrap();
        assert_eq!(idx, 2);
    }

    #[test]
    fn boundary_strength_uses_particle_symbol_or_long_right() {
        assert!(is_boundary_strong(
            &token("を", "助詞"),
            &token("実装", "名詞")
        ));
        assert!(is_boundary_strong(
            &token("。", "記号"),
            &token("次", "名詞")
        ));
        assert!(is_boundary_strong(
            &token("テ", "名詞"),
            &token("タイトル", "名詞")
        ));
        assert!(!is_boundary_strong(
            &token("短", "名詞"),
            &token("文", "名詞")
        ));
    }

    #[test]
    fn particle_detection_works_by_pos_or_surface() {
        assert!(is_particle(&token("X", "助詞")));
        assert!(is_particle(&token("を", "名詞")));
        assert!(!is_particle(&token("自動", "名詞")));
    }

    #[test]
    fn lindera_tokenizer_returns_non_empty_units() {
        let units = tokenize_with_lindera("形態素解析で改行する").unwrap();
        assert!(!units.is_empty());
        assert!(units.iter().all(|u| !u.surface.is_empty()));
    }

    fn token(surface: &str, pos_major: &str) -> TokenUnit {
        TokenUnit {
            surface: surface.to_string(),
            pos_major: pos_major.to_string(),
        }
    }

    fn surfaces(tokens: &[TokenUnit]) -> Vec<&str> {
        tokens.iter().map(|t| t.surface.as_str()).collect()
    }
}
