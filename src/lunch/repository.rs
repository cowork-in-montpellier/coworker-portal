use chrono::{NaiveDate, Utc};
use sqlx::SqlitePool;

use super::domain::ValidPlace;

/// A place with its members' ratings, as seen by one member.
#[derive(Debug, sqlx::FromRow)]
pub struct PlaceRow {
    pub id: i64,
    pub name: String,
    pub cuisine: String,
    pub address: String,
    pub maps_url: Option<String>,
    pub open_days: i64,
    pub under_ten_euros: bool,
    pub veggie_friendly: bool,
    pub phone: Option<String>,
    pub description: String,
    pub is_gamelle: bool,
    pub average_rating: Option<f64>,
    pub rating_count: i64,
    pub my_rating: Option<i64>,
}

/// Every place, the "J'ai ma gamelle" entry first, then by name.
pub async fn list_places(db: &SqlitePool, user_id: i32) -> Result<Vec<PlaceRow>, sqlx::Error> {
    sqlx::query_as::<_, PlaceRow>(
        "SELECT p.id, p.name, p.cuisine, p.address, p.maps_url, p.open_days, p.under_ten_euros, p.veggie_friendly, p.phone, p.description, p.is_gamelle,
                (SELECT AVG(stars) FROM rating r WHERE r.place_id = p.id) AS average_rating,
                (SELECT COUNT(*) FROM rating r WHERE r.place_id = p.id) AS rating_count,
                (SELECT stars FROM rating r WHERE r.place_id = p.id AND r.user_id = $1) AS my_rating
         FROM place p
         ORDER BY p.is_gamelle DESC, p.name COLLATE NOCASE",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// Whether a place exists, and if so whether it is the "J'ai ma gamelle" entry.
pub async fn is_gamelle(db: &SqlitePool, id: i64) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar("SELECT is_gamelle FROM place WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn create_place(db: &SqlitePool, place: &ValidPlace, user_id: i32) -> Result<i64, sqlx::Error> {
    let now = Utc::now();
    sqlx::query_scalar(
        "INSERT INTO place
             (name, cuisine, address, maps_url, open_days, under_ten_euros, veggie_friendly, phone, description,
              created_by, created_at, updated_by, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $10, $11)
         RETURNING id",
    )
    .bind(&place.name)
    .bind(&place.cuisine)
    .bind(&place.address)
    .bind(&place.maps_url)
    .bind(place.open_days)
    .bind(place.under_ten_euros)
    .bind(place.veggie_friendly)
    .bind(&place.phone)
    .bind(&place.description)
    .bind(user_id)
    .bind(now)
    .fetch_one(db)
    .await
}

/// Updates a place; `false` if there is no such place (the gamelle entry can't be changed).
pub async fn update_place(db: &SqlitePool, id: i64, place: &ValidPlace, user_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE place
         SET name = $1, cuisine = $2, address = $3, maps_url = $4, open_days = $5, under_ten_euros = $6,
             veggie_friendly = $7, phone = $8, description = $9, updated_by = $10, updated_at = $11
         WHERE id = $12 AND is_gamelle = 0",
    )
    .bind(&place.name)
    .bind(&place.cuisine)
    .bind(&place.address)
    .bind(&place.maps_url)
    .bind(place.open_days)
    .bind(place.under_ten_euros)
    .bind(place.veggie_friendly)
    .bind(&place.phone)
    .bind(&place.description)
    .bind(user_id)
    .bind(Utc::now())
    .bind(id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Deletes a place with its ratings and intentions; `false` if there is no such place
/// (the gamelle entry can't be deleted).
pub async fn delete_place(db: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM place WHERE id = $1 AND is_gamelle = 0").bind(id).execute(db).await?;
    Ok(result.rows_affected() > 0)
}

/// Sets a member's rating of a place, or removes it with `None`.
pub async fn set_rating(db: &SqlitePool, place_id: i64, user_id: i32, stars: Option<i64>) -> Result<(), sqlx::Error> {
    match stars {
        Some(stars) => {
            sqlx::query(
                "INSERT INTO rating (place_id, user_id, stars, updated_at) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (place_id, user_id) DO UPDATE SET stars = excluded.stars, updated_at = excluded.updated_at",
            )
            .bind(place_id)
            .bind(user_id)
            .bind(stars)
            .bind(Utc::now())
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM rating WHERE place_id = $1 AND user_id = $2")
                .bind(place_id)
                .bind(user_id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}

/// Sets where a member has lunch on a day, replacing any earlier choice, or clears it with `None`.
pub async fn set_intention(
    db: &SqlitePool,
    user_id: i32,
    date: NaiveDate,
    place_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    match place_id {
        Some(place_id) => {
            sqlx::query(
                "INSERT INTO intention (user_id, lunch_date, place_id, created_at) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (user_id, lunch_date) DO UPDATE SET place_id = excluded.place_id, created_at = excluded.created_at",
            )
            .bind(user_id)
            .bind(date)
            .bind(place_id)
            .bind(Utc::now())
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM intention WHERE user_id = $1 AND lunch_date = $2")
                .bind(user_id)
                .bind(date)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}

/// Who means to have lunch where on a day, as `(user_id, place_id)`, earliest choice first.
pub async fn intentions_on(db: &SqlitePool, date: NaiveDate) -> Result<Vec<(i32, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT user_id, place_id FROM intention WHERE lunch_date = $1 ORDER BY created_at")
        .bind(date)
        .fetch_all(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lunch::db;

    fn place(name: &str) -> ValidPlace {
        ValidPlace {
            name: name.into(),
            cuisine: "Burgers".into(),
            address: "1 rue du Test".into(),
            maps_url: None,
            open_days: 31,
            under_ten_euros: false,
            veggie_friendly: false,
            phone: None,
            description: String::new(),
        }
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    async fn gamelle_id(db: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT id FROM place WHERE is_gamelle = 1").fetch_one(db).await.unwrap()
    }

    #[tokio::test]
    async fn seed_lists_the_gamelle_first() {
        let db = db::in_memory().await;
        let places = list_places(&db, 1).await.unwrap();
        assert_eq!(places.len(), 23);
        assert!(places[0].is_gamelle);
        assert!(places[1..].iter().all(|p| !p.is_gamelle));
    }

    #[tokio::test]
    async fn seed_fills_in_maps_links_and_days() {
        let db = db::in_memory().await;
        let places = list_places(&db, 1).await.unwrap();
        let find = |name: &str| places.iter().find(|p| p.name == name).unwrap();

        assert!(places.iter().all(|p| p.is_gamelle || p.maps_url.is_some()));
        assert_eq!(find("Chez Gaby").open_days, 2 | 4 | 8 | 16); // closed on Mondays

        let habibi = find("Habibi");
        assert_eq!(habibi.open_days, 1 | 2 | 8 | 16); // closed on Wednesdays
        assert_eq!(habibi.maps_url.as_deref(), Some("https://maps.app.goo.gl/ypgW9GHQte4wN5k56"));
        assert!(find("L'oignon givré").under_ten_euros);
        let veggie: Vec<&str> = places.iter().filter(|p| p.veggie_friendly).map(|p| p.name.as_str()).collect();
        assert_eq!(veggie, vec!["KVB", "L'oignon givré"]);
        assert!(places.iter().all(|p| p.is_gamelle || p.phone.is_some()));
        assert_eq!(find("Coco d'Asie").phone.as_deref(), Some("04 67 64 83 64"));
        assert_eq!(find("Jimmy Gourmet").open_days, 4 | 16);
    }

    #[tokio::test]
    async fn places_are_created_updated_and_deleted() {
        let db = db::in_memory().await;
        let id = create_place(&db, &place("Zinc"), 7).await.unwrap();
        assert_eq!(is_gamelle(&db, id).await.unwrap(), Some(false));

        let changed = ValidPlace {
            under_ten_euros: true,
            veggie_friendly: true,
            phone: Some("04 67 12 34 56".into()),
            description: "Terrasse au soleil".into(),
            ..place("Zinc bar")
        };
        assert!(update_place(&db, id, &changed, 8).await.unwrap());
        let listed = list_places(&db, 7).await.unwrap();
        assert!(
            listed
                .iter()
                .any(|p| p.name == "Zinc bar" && p.under_ten_euros && p.veggie_friendly && p.phone.is_some() && p.description == "Terrasse au soleil")
        );

        set_rating(&db, id, 7, Some(4)).await.unwrap();
        set_intention(&db, 7, date(27), Some(id)).await.unwrap();
        assert!(delete_place(&db, id).await.unwrap());
        assert_eq!(is_gamelle(&db, id).await.unwrap(), None);
        // Ratings and intentions go with the place.
        assert!(intentions_on(&db, date(27)).await.unwrap().is_empty());
        let ratings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rating").fetch_one(&db).await.unwrap();
        assert_eq!(ratings, 0);
    }

    #[tokio::test]
    async fn the_gamelle_cannot_be_changed_or_deleted() {
        let db = db::in_memory().await;
        let id = gamelle_id(&db).await;
        assert_eq!(is_gamelle(&db, id).await.unwrap(), Some(true));
        assert!(!update_place(&db, id, &place("Autre"), 7).await.unwrap());
        assert!(!delete_place(&db, id).await.unwrap());
    }

    #[tokio::test]
    async fn one_rating_per_member_averaged() {
        let db = db::in_memory().await;
        let id = create_place(&db, &place("Zinc"), 7).await.unwrap();
        set_rating(&db, id, 7, Some(2)).await.unwrap();
        set_rating(&db, id, 7, Some(4)).await.unwrap();
        set_rating(&db, id, 8, Some(5)).await.unwrap();

        let zinc = list_places(&db, 7).await.unwrap().into_iter().find(|p| p.id == id).unwrap();
        assert_eq!(zinc.rating_count, 2);
        assert_eq!(zinc.average_rating, Some(4.5));
        assert_eq!(zinc.my_rating, Some(4));

        set_rating(&db, id, 7, None).await.unwrap();
        let zinc = list_places(&db, 7).await.unwrap().into_iter().find(|p| p.id == id).unwrap();
        assert_eq!(zinc.rating_count, 1);
        assert_eq!(zinc.my_rating, None);
    }

    #[tokio::test]
    async fn one_intention_per_member_and_day() {
        let db = db::in_memory().await;
        let zinc = create_place(&db, &place("Zinc"), 7).await.unwrap();
        let gamelle = gamelle_id(&db).await;

        set_intention(&db, 7, date(27), Some(zinc)).await.unwrap();
        set_intention(&db, 8, date(27), Some(zinc)).await.unwrap();
        set_intention(&db, 7, date(27), Some(gamelle)).await.unwrap();
        set_intention(&db, 7, date(26), Some(zinc)).await.unwrap();

        let mut today = intentions_on(&db, date(27)).await.unwrap();
        today.sort();
        assert_eq!(today, vec![(7, gamelle), (8, zinc)]);

        set_intention(&db, 8, date(27), None).await.unwrap();
        assert_eq!(intentions_on(&db, date(27)).await.unwrap(), vec![(7, gamelle)]);
    }
}
