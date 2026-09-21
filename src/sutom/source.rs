use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::NaiveDate;

use super::domain::clean_word;

const BASE_URL: &str = "https://sutom.nocle.fr";
// The public instance's default game id (constant across all players), taken from
// `js/instanceConfiguration.js` on the source site.
const DEFAULT_GAME_ID: &str = "34ccc522-c264-4e51-b293-fd5bd60ef7aa";

/// Reproduces `Dictionnaire.getNomFichier` from the source site's `js/dictionnaire.js`:
/// the word-of-the-day file name is `base64("{gameId}-{YYYY-MM-DD}")`.
fn word_of_day_file_name(date: NaiveDate) -> String {
    let raw = format!("{DEFAULT_GAME_ID}-{}", date.format("%Y-%m-%d"));
    BASE64.encode(raw.as_bytes())
}

pub async fn fetch_word_of_day(client: &reqwest::Client, date: NaiveDate) -> Result<String> {
    let file_name = word_of_day_file_name(date);
    let url = format!("{BASE_URL}/mots/{file_name}.txt");

    let resp = client.get(&url).send().await.context("fetching SUTOM word of the day")?;
    if !resp.status().is_success() {
        anyhow::bail!("SUTOM word of the day not published yet (HTTP {})", resp.status());
    }
    let raw = resp.text().await.context("reading SUTOM word of the day response")?;
    let word = clean_word(&raw);
    if word.is_empty() {
        anyhow::bail!("SUTOM word of the day response was empty");
    }
    Ok(word)
}

/// Fetches the guessable-words dictionary for a given word length and first letter, e.g.
/// `js/mots/listeMotsProposables.7.B.js`. Every valid SUTOM guess shares the target word's
/// length and first letter, so this single list is enough to validate all guesses for the day.
pub async fn fetch_possible_words(
    client: &reqwest::Client,
    length: usize,
    first_letter: char,
) -> Result<Vec<String>> {
    let url = format!("{BASE_URL}/js/mots/listeMotsProposables.{length}.{first_letter}.js");
    let resp = client.get(&url).send().await.context("fetching SUTOM word list")?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "SUTOM word list not found for length={length} first_letter={first_letter} (HTTP {})",
            resp.status()
        );
    }
    let body = resp.text().await.context("reading SUTOM word list response")?;
    parse_word_list(&body)
}

/// Extracts the quoted words out of the AMD module body:
/// `ListeMotsProposables.Dictionnaire = ["MOT1", "MOT2", ...];`
fn parse_word_list(body: &str) -> Result<Vec<String>> {
    const MARKER: &str = "Dictionnaire = [";
    let marker_pos = body.find(MARKER).context("SUTOM word list: missing Dictionnaire array")?;
    let array_start = marker_pos + MARKER.len() - 1;
    let array_end = body.rfind(']').context("SUTOM word list: missing closing bracket")?;
    if array_end <= array_start {
        anyhow::bail!("SUTOM word list: malformed array bounds");
    }

    let words: Vec<String> = body[array_start + 1..array_end]
        .split(',')
        .filter_map(|entry| {
            let word = clean_word(entry.trim().trim_matches('"'));
            if word.is_empty() { None } else { Some(word) }
        })
        .collect();

    if words.is_empty() {
        anyhow::bail!("SUTOM word list: no words parsed");
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_of_day_file_name_matches_reference_encoding() {
        // Verified against the live site on 2026-09-21 (word was "BLOUSON").
        let date = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        assert_eq!(
            word_of_day_file_name(date),
            "MzRjY2M1MjItYzI2NC00ZTUxLWIyOTMtZmQ1YmQ2MGVmN2FhLTIwMjYtMDktMjE=",
        );
    }

    #[test]
    fn parse_word_list_extracts_words() {
        let body = r#"
            define(["require", "exports"], factory);
            var ListeMotsProposables = (function () {
                ListeMotsProposables.Dictionnaire = [
                    "BABACHE",
                    "BABELAI",
                ];
                return ListeMotsProposables;
            }());
        "#;
        assert_eq!(parse_word_list(body).unwrap(), vec!["BABACHE", "BABELAI"]);
    }

    #[test]
    fn parse_word_list_rejects_missing_array() {
        assert!(parse_word_list("no array here").is_err());
    }
}
