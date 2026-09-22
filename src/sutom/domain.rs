use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// Number of guesses a player gets before the puzzle is marked as failed.
pub const MAX_ATTEMPTS: usize = 6;
/// Score recorded when a player uses all attempts without finding the word.
pub const FAILED_SCORE: i32 = MAX_ATTEMPTS as i32 + 1;

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

/// The reference site's own day numbering ("SUTOM #N"): N days since its 2022-01-08
/// launch, taken from `js/finDePartiePanel.js`'s `numeroGrille` computation.
pub fn puzzle_number(date: NaiveDate) -> i32 {
    let origin = NaiveDate::from_ymd_opt(2022, 1, 8).expect("valid origin date");
    (date - origin).num_days() as i32 + 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LetterStatus {
    Correct,
    Present,
    Absent,
}

/// Mirrors the source site's `analyserMot`: exact matches are resolved first, then
/// leftover letter counts are handed out left-to-right to misplaced-letter guesses.
pub fn score_guess(target: &str, guess: &str) -> Vec<LetterStatus> {
    let target_letters: Vec<char> = target.chars().collect();
    let guess_letters: Vec<char> = guess.chars().collect();
    let mut remaining: HashMap<char, i32> = HashMap::new();

    for (i, &c) in target_letters.iter().enumerate() {
        if guess_letters.get(i) != Some(&c) {
            *remaining.entry(c).or_insert(0) += 1;
        }
    }

    guess_letters
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if target_letters.get(i) == Some(&c) {
                LetterStatus::Correct
            } else if let Some(count) = remaining.get_mut(&c) {
                if *count > 0 {
                    *count -= 1;
                    LetterStatus::Present
                } else {
                    LetterStatus::Absent
                }
            } else {
                LetterStatus::Absent
            }
        })
        .collect()
}

/// The number of attempts a solved puzzle took (1..=MAX_ATTEMPTS), `FAILED_SCORE` if all
/// attempts were used without solving it, or `None` if the game is still in progress.
pub fn score_for(target: &str, guesses: &[String]) -> Option<i32> {
    if let Some(pos) = guesses.iter().position(|g| g == target) {
        return Some(pos as i32 + 1);
    }
    if guesses.len() >= MAX_ATTEMPTS {
        return Some(FAILED_SCORE);
    }
    None
}

/// Validates a single guess against the day's word shape and dictionary, mirroring the
/// source site's own UX messages.
pub fn validate_guess(target: &str, possible_words: &[String], guess: &str) -> Result<(), String> {
    if guess.chars().count() != target.chars().count() {
        return Err("Le mot proposé n'a pas la bonne longueur.".into());
    }
    if guess.chars().next() != target.chars().next() {
        return Err(
            "Le mot proposé doit commencer par la même lettre que le mot recherché.".into(),
        );
    }
    if !possible_words.iter().any(|w| w == guess) {
        return Err("Ce mot n'est pas dans notre dictionnaire.".into());
    }
    Ok(())
}

/// Stableford-like scoring for the 30-day leaderboard: matching par is worth 3 points,
/// and each attempt better/worse than par shifts the score, with overshoots (a worse
/// score than par) penalized more gently than undershoots are rewarded. Being the first
/// to finish that day adds a small bonus, and catching up on a day that's already past
/// only earns half of what it would have on the day itself. Never finding the word at
/// all is worth nothing, no matter how bad everyone else's par was — failing to solve it
/// isn't just "a bad score", so it never earns points or bonuses.
pub fn leaderboard_points(par: i32, score: i32, first_to_finish: bool, is_catchup: bool) -> f64 {
    if score >= FAILED_SCORE {
        return 0.0;
    }
    let diff = (par - score) as f64;
    let divisor = if score > par { 2.0 } else { 1.0 };
    let base = 3.0 + diff / divisor + if first_to_finish { 0.5 } else { 0.0 };
    if is_catchup { base * 0.5 } else { base }
}

/// True if a day's puzzle was completed after the day itself (a later catch-up play),
/// rather than on the day it was published, both compared in Paris local time.
pub fn is_catchup_play(game_date: NaiveDate, completed_at: DateTime<Utc>) -> bool {
    completed_at.with_timezone(&chrono_tz::Europe::Paris).date_naive() != game_date
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_accents_and_uppercases() {
        assert_eq!(clean_word("blouson"), "BLOUSON");
        assert_eq!(clean_word("Écœuré "), "ECOURE");
    }

    #[test]
    fn puzzle_number_matches_reference_site() {
        // Confirmed against the live site on 2026-09-21: "SUTOM #1718".
        let date = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        assert_eq!(puzzle_number(date), 1718);
    }

    #[test]
    fn score_guess_handles_duplicate_letters() {
        // Target BLOUSON, guess BABACHE: only the leading B is correct; the second B
        // in the guess must not consume the already-matched target B.
        let result = score_guess("BLOUSON", "BABACHE");
        assert_eq!(
            result,
            vec![
                LetterStatus::Correct,
                LetterStatus::Absent,
                LetterStatus::Absent,
                LetterStatus::Absent,
                LetterStatus::Absent,
                LetterStatus::Absent,
                LetterStatus::Absent,
            ]
        );
    }

    #[test]
    fn score_for_tracks_progress_and_outcome() {
        let guesses = vec!["BABACHE".to_string()];
        assert_eq!(score_for("BLOUSON", &guesses), None);

        let solved = vec!["BABACHE".to_string(), "BLOUSON".to_string()];
        assert_eq!(score_for("BLOUSON", &solved), Some(2));

        let failed: Vec<String> = (0..6).map(|_| "BABACHE".to_string()).collect();
        assert_eq!(score_for("BLOUSON", &failed), Some(FAILED_SCORE));
    }

    #[test]
    fn leaderboard_points_matches_spec_formula() {
        assert_eq!(leaderboard_points(4, 4, false, false), 3.0);
        assert_eq!(leaderboard_points(4, 1, false, false), 6.0);
        assert_eq!(leaderboard_points(4, 6, false, false), 2.0);
    }

    #[test]
    fn leaderboard_points_never_rewards_a_failed_attempt() {
        // Not finding the word (FAILED_SCORE) always earns zero — not finding it isn't
        // just "a very bad score", regardless of how forgiving par is, the bonus, or catchup.
        assert_eq!(leaderboard_points(4, FAILED_SCORE, false, false), 0.0);
        assert_eq!(leaderboard_points(7, FAILED_SCORE, false, false), 0.0);
        assert_eq!(leaderboard_points(4, FAILED_SCORE, true, false), 0.0);
        assert_eq!(leaderboard_points(4, FAILED_SCORE, false, true), 0.0);
        assert_eq!(leaderboard_points(4, FAILED_SCORE, true, true), 0.0);
    }

    #[test]
    fn leaderboard_points_adds_first_to_finish_bonus() {
        assert_eq!(leaderboard_points(4, 4, true, false), 3.5);
        assert_eq!(leaderboard_points(4, 1, true, false), 6.5);
    }

    #[test]
    fn leaderboard_points_halves_catchup_plays() {
        // Same performance, but played after the day itself: half the points, bonus included.
        assert_eq!(leaderboard_points(4, 4, false, true), 1.5);
        assert_eq!(leaderboard_points(4, 4, true, true), 1.75);
    }

    #[test]
    fn is_catchup_play_compares_paris_local_dates() {
        let game_date = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        let same_day = NaiveDate::from_ymd_opt(2026, 9, 21)
            .unwrap()
            .and_hms_opt(20, 0, 0)
            .unwrap()
            .and_utc();
        let next_day = NaiveDate::from_ymd_opt(2026, 9, 22)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!is_catchup_play(game_date, same_day));
        assert!(is_catchup_play(game_date, next_day));
    }
}
