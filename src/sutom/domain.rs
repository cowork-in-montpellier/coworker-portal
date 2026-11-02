use std::collections::HashMap;

use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
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

/// The day the general leaderboard competition officially started. Earlier days are
/// playable (warm-up/training) but never count toward leaderboard points.
pub fn leaderboard_start_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 23).expect("valid leaderboard start date")
}

/// The first month whose leaderboard winner earns a star.
pub fn first_titled_month() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 1).expect("valid first titled month")
}

/// First and last day of the calendar month containing `date`.
pub fn month_bounds(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let first = date.with_day(1).expect("day 1 exists in every month");
    let next_first = first.checked_add_months(Months::new(1)).expect("month within range");
    (first, next_first.pred_opt().expect("day before a month start exists"))
}

/// First letter of a last name, uppercased (e.g. to disambiguate two players sharing a
/// first name in the leaderboards), or empty if the last name itself is empty.
pub fn last_initial(last_name: &str) -> String {
    last_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
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

/// Under this time between the first guess and solving it, a game earns the speed bonus.
pub const FAST_FINISH_SECS: i64 = 60;

/// Stableford-like scoring for the monthly leaderboard: matching par is worth 3 points,
/// and each attempt better/worse than par shifts the score, with overshoots (a worse
/// score than par) penalized more gently than undershoots are rewarded. Being the first
/// to finish that day and solving it in under a minute each add a small bonus, and the
/// total (bonuses included) is scaled down when the day is played late. Never finding the
/// word at all is worth nothing, no matter how bad everyone else's par was — failing to
/// solve it isn't just "a bad score", so it never earns points or bonuses.
pub fn leaderboard_points(par: i32, score: i32, first_to_finish: bool, fast: bool, days_late: i32) -> f64 {
    if score >= FAILED_SCORE {
        return 0.0;
    }
    let diff = (par - score) as f64;
    let divisor = if score > par { 2.0 } else { 1.0 };
    let bonus = if first_to_finish { 0.5 } else { 0.0 } + if fast { 0.5 } else { 0.0 };
    (3.0 + diff / divisor + bonus) * late_multiplier(days_late)
}

/// How many Paris-local days after the puzzle's own day it was completed (0 = on the day).
pub fn days_late(game_date: NaiveDate, completed_at: DateTime<Utc>) -> i32 {
    let completed_on = completed_at.with_timezone(&chrono_tz::Europe::Paris).date_naive();
    (completed_on - game_date).num_days().max(0) as i32
}

/// Share of the points kept when a day is played late: all of it on the day, 75 % the
/// next day, 50 % beyond.
pub fn late_multiplier(days_late: i32) -> f64 {
    match days_late {
        0 => 1.0,
        1 => 0.75,
        _ => 0.5,
    }
}

/// True if the word was found less than `FAST_FINISH_SECS` after the first guess (so a
/// first-guess win always qualifies). Attempts with no recorded start (begun before start times were tracked) never qualify.
pub fn is_fast_finish(started_at: Option<DateTime<Utc>>, finished_at: DateTime<Utc>, score: i32) -> bool {
    score < FAILED_SCORE
        && started_at.is_some_and(|start| (finished_at - start).num_seconds() < FAST_FINISH_SECS)
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

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, s).unwrap().and_utc()
    }

    #[test]
    fn leaderboard_points_matches_spec_formula() {
        assert_eq!(leaderboard_points(4, 4, false, false, 0), 3.0);
        assert_eq!(leaderboard_points(4, 1, false, false, 0), 6.0);
        assert_eq!(leaderboard_points(4, 6, false, false, 0), 2.0);
    }

    #[test]
    fn leaderboard_points_never_rewards_a_failed_attempt() {
        // Not finding the word (FAILED_SCORE) always earns zero — not finding it isn't
        // just "a very bad score", regardless of how forgiving par is, bonuses, or lateness.
        assert_eq!(leaderboard_points(4, FAILED_SCORE, false, false, 0), 0.0);
        assert_eq!(leaderboard_points(7, FAILED_SCORE, false, false, 0), 0.0);
        assert_eq!(leaderboard_points(4, FAILED_SCORE, true, true, 0), 0.0);
        assert_eq!(leaderboard_points(4, FAILED_SCORE, true, true, 3), 0.0);
    }

    #[test]
    fn leaderboard_points_adds_bonuses() {
        assert_eq!(leaderboard_points(4, 4, true, false, 0), 3.5);
        assert_eq!(leaderboard_points(4, 4, false, true, 0), 3.5);
        assert_eq!(leaderboard_points(4, 1, true, true, 0), 7.0);
    }

    #[test]
    fn leaderboard_points_scales_late_plays_bonuses_included() {
        assert_eq!(leaderboard_points(4, 4, false, false, 1), 2.25);
        assert_eq!(leaderboard_points(4, 4, true, false, 1), 2.625);
        assert_eq!(leaderboard_points(4, 4, false, false, 2), 1.5);
        assert_eq!(leaderboard_points(4, 4, true, false, 10), 1.75);
    }

    #[test]
    fn days_late_compares_paris_local_dates() {
        let game_date = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        assert_eq!(days_late(game_date, utc(2026, 9, 21, 20, 0, 0)), 0);
        // 22:30 UTC on the 21st is already 00:30 on the 22nd in Paris (summer time).
        assert_eq!(days_late(game_date, utc(2026, 9, 21, 22, 30, 0)), 1);
        assert_eq!(days_late(game_date, utc(2026, 9, 22, 10, 0, 0)), 1);
        assert_eq!(days_late(game_date, utc(2026, 9, 25, 10, 0, 0)), 4);
    }

    #[test]
    fn is_fast_finish_requires_a_solve_within_a_minute_of_opening() {
        let start = utc(2026, 9, 21, 9, 0, 0);
        assert!(is_fast_finish(Some(start), utc(2026, 9, 21, 9, 0, 59), 3));
        assert!(!is_fast_finish(Some(start), utc(2026, 9, 21, 9, 1, 0), 3));
        assert!(!is_fast_finish(Some(start), utc(2026, 9, 21, 9, 0, 30), FAILED_SCORE));
        assert!(!is_fast_finish(None, utc(2026, 9, 21, 9, 0, 30), 3));
    }

    #[test]
    fn month_bounds_covers_the_whole_month() {
        let (first, last) = month_bounds(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
        assert_eq!(first, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(last, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        let (_, last) = month_bounds(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
        assert_eq!(last, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
    }
}
