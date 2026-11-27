use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A weekday the coworking is open, and so a place can be open for lunch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
}

impl Weekday {
    pub const ALL: [Weekday; 5] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];

    fn bit(self) -> i64 {
        1 << (self as i64)
    }

    /// The weekday of a date, or `None` on a weekend.
    pub fn of(date: NaiveDate) -> Option<Weekday> {
        match date.weekday() {
            chrono::Weekday::Mon => Some(Weekday::Mon),
            chrono::Weekday::Tue => Some(Weekday::Tue),
            chrono::Weekday::Wed => Some(Weekday::Wed),
            chrono::Weekday::Thu => Some(Weekday::Thu),
            chrono::Weekday::Fri => Some(Weekday::Fri),
            chrono::Weekday::Sat | chrono::Weekday::Sun => None,
        }
    }
}

/// The stored bit mask of a set of weekdays (Mon = 1 … Fri = 16).
pub fn days_to_mask(days: &[Weekday]) -> i64 {
    days.iter().fold(0, |mask, d| mask | d.bit())
}

/// The weekdays of a stored bit mask, in week order.
pub fn mask_to_days(mask: i64) -> Vec<Weekday> {
    Weekday::ALL.into_iter().filter(|d| mask & d.bit() != 0).collect()
}

/// Whether a place with these open days serves lunch on `date`. Never on weekends.
pub fn is_open_on(mask: i64, date: NaiveDate) -> bool {
    Weekday::of(date).is_some_and(|d| mask & d.bit() != 0)
}

/// Today, Paris time: the day intentions are declared for.
pub fn today() -> NaiveDate {
    chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris).date_naive()
}

/// First letter of a last name, uppercased, or empty if the last name itself is empty.
pub fn last_initial(last_name: &str) -> String {
    last_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
}

pub const NAME_MAX_LEN: usize = 80;
pub const CUISINE_MAX_LEN: usize = 40;
pub const ADDRESS_MAX_LEN: usize = 200;
pub const MAPS_URL_MAX_LEN: usize = 500;
pub const PHONE_MAX_LEN: usize = 30;
pub const DESCRIPTION_MAX_LEN: usize = 500;

/// A place as typed in by a member, before it is saved.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PlaceInput {
    pub name: String,
    #[serde(default)]
    pub cuisine: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub maps_url: Option<String>,
    pub open_days: Vec<Weekday>,
    /// Whether one can have lunch there for less than 10 €.
    #[serde(default)]
    pub under_ten_euros: bool,
    /// Whether vegetarians find something good to eat there.
    #[serde(default)]
    pub veggie_friendly: bool,
    /// A phone number to call the place, as typed in.
    #[serde(default)]
    pub phone: Option<String>,
    /// A few words about the place, written by members.
    #[serde(default)]
    pub description: String,
}

/// A place checked and trimmed, ready to be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidPlace {
    pub name: String,
    pub cuisine: String,
    pub address: String,
    pub maps_url: Option<String>,
    pub open_days: i64,
    pub under_ten_euros: bool,
    pub veggie_friendly: bool,
    pub phone: Option<String>,
    pub description: String,
}

/// Checks a place typed in by a member. Errors are messages for the member.
pub fn validate_place(input: PlaceInput) -> Result<ValidPlace, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Donne un nom à cet endroit.".into());
    }
    if name.chars().count() > NAME_MAX_LEN {
        return Err(format!("Le nom fait plus de {NAME_MAX_LEN} caractères."));
    }

    let cuisine = input.cuisine.trim().to_string();
    if cuisine.chars().count() > CUISINE_MAX_LEN {
        return Err(format!("Le type fait plus de {CUISINE_MAX_LEN} caractères."));
    }

    let address = input.address.trim().to_string();
    if address.chars().count() > ADDRESS_MAX_LEN {
        return Err(format!("L'adresse fait plus de {ADDRESS_MAX_LEN} caractères."));
    }

    let maps_url = input.maps_url.map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
    if let Some(url) = &maps_url
        && (!(url.starts_with("https://") || url.starts_with("http://")) || url.len() > MAPS_URL_MAX_LEN)
    {
        return Err("Le lien Google Maps doit commencer par https://".into());
    }

    let phone = input.phone.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    if let Some(phone) = &phone
        && !is_valid_phone(phone)
    {
        return Err("Le numéro de téléphone n'est pas valide.".into());
    }

    let description = input.description.trim().to_string();
    if description.chars().count() > DESCRIPTION_MAX_LEN {
        return Err(format!("La description fait plus de {DESCRIPTION_MAX_LEN} caractères."));
    }

    let open_days = days_to_mask(&input.open_days);
    if open_days == 0 {
        return Err("Coche au moins un jour d'ouverture.".into());
    }

    Ok(ValidPlace {
        name,
        cuisine,
        address,
        maps_url,
        open_days,
        under_ten_euros: input.under_ten_euros,
        veggie_friendly: input.veggie_friendly,
        phone,
        description,
    })
}

/// A phone number as people write it: digits, spaces and `+ . - ( )`, with at least 6 digits.
fn is_valid_phone(phone: &str) -> bool {
    phone.len() <= PHONE_MAX_LEN
        && phone.chars().all(|c| c.is_ascii_digit() || " +.-()".contains(c))
        && phone.chars().filter(char::is_ascii_digit).count() >= 6
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_DAYS_MASK: i64 = 31;

    fn input(name: &str, days: &[Weekday]) -> PlaceInput {
        PlaceInput {
            name: name.into(),
            cuisine: " Burgers ".into(),
            address: String::new(),
            maps_url: Some("  ".into()),
            open_days: days.to_vec(),
            under_ten_euros: true,
            veggie_friendly: true,
            phone: Some(" 04 67 12 34 56 ".into()),
            description: "  Formule midi à 11 €  ".into(),
        }
    }

    #[test]
    fn days_round_trip_through_the_mask() {
        assert_eq!(days_to_mask(&Weekday::ALL), ALL_DAYS_MASK);
        assert_eq!(days_to_mask(&[Weekday::Wed, Weekday::Fri]), 20);
        assert_eq!(mask_to_days(20), vec![Weekday::Wed, Weekday::Fri]);
        assert_eq!(mask_to_days(ALL_DAYS_MASK), Weekday::ALL.to_vec());
    }

    #[test]
    fn open_only_on_its_days_and_never_on_weekends() {
        let wednesday = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let saturday = NaiveDate::from_ymd_opt(2026, 11, 28).unwrap();
        assert!(is_open_on(20, wednesday));
        assert!(!is_open_on(20, thursday));
        assert!(!is_open_on(ALL_DAYS_MASK, saturday));
    }

    #[test]
    fn validation_trims_and_drops_an_empty_maps_link() {
        let place = validate_place(input("  Habibi ", &[Weekday::Mon])).unwrap();
        assert_eq!(place.name, "Habibi");
        assert_eq!(place.cuisine, "Burgers");
        assert_eq!(place.maps_url, None);
        assert_eq!(place.open_days, 1);
        assert!(place.under_ten_euros);
        assert!(place.veggie_friendly);
        assert_eq!(place.phone.as_deref(), Some("04 67 12 34 56"));
        assert_eq!(place.description, "Formule midi à 11 €");
    }

    #[test]
    fn validation_rejects_bad_places() {
        assert!(validate_place(input("   ", &[Weekday::Mon])).is_err());
        assert!(validate_place(input("Habibi", &[])).is_err());
        assert!(validate_place(input(&"x".repeat(NAME_MAX_LEN + 1), &[Weekday::Mon])).is_err());

        let mut bad_link = input("Habibi", &[Weekday::Mon]);
        bad_link.maps_url = Some("javascript:alert(1)".into());
        assert!(validate_place(bad_link).is_err());

        let mut long = input("Habibi", &[Weekday::Mon]);
        long.description = "x".repeat(DESCRIPTION_MAX_LEN + 1);
        assert!(validate_place(long).is_err());

        for phone in ["12 34", "04 67 12 34 56 <script>", "tel:0467123456"] {
            let mut bad_phone = input("Habibi", &[Weekday::Mon]);
            bad_phone.phone = Some(phone.into());
            assert!(validate_place(bad_phone).is_err(), "{phone}");
        }
    }
}
