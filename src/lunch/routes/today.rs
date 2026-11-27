use std::collections::BTreeSet;

use axum::{Json, extract::State};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::lunch::domain::{self, Weekday};
use crate::lunch::{repository, state::State as LunchState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct Goer {
    pub user_id: i32,
    pub first_name: String,
    pub last_initial: String,
}

#[derive(Serialize, ToSchema)]
pub struct PlaceResponse {
    pub id: i64,
    pub name: String,
    pub cuisine: String,
    pub address: String,
    pub maps_url: Option<String>,
    pub open_days: Vec<Weekday>,
    /// Whether one can have lunch there for less than 10 €.
    pub under_ten_euros: bool,
    /// Whether vegetarians find something good to eat there.
    pub veggie_friendly: bool,
    /// A phone number to call the place, as typed in.
    pub phone: Option<String>,
    /// A few words about the place, written by members; may be empty.
    pub description: String,
    /// The "J'ai ma gamelle" entry: can't be rated, changed or deleted.
    pub is_gamelle: bool,
    pub open_today: bool,
    pub average_rating: Option<f64>,
    pub rating_count: i64,
    pub my_rating: Option<i64>,
    /// Who means to have lunch there today, earliest choice first.
    pub goers: Vec<Goer>,
}

#[derive(Serialize, ToSchema)]
pub struct TodayResponse {
    /// Today, Paris time.
    pub date: NaiveDate,
    /// Today's weekday, or `null` on a weekend.
    pub weekday: Option<Weekday>,
    pub places: Vec<PlaceResponse>,
    /// The types of cuisine in use, for suggestions and filters.
    pub cuisines: Vec<String>,
    /// Where the current member means to have lunch today, if they said so.
    pub my_place_id: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/lunch/today",
    tag = "Lunch",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Every place, with ratings and who goes where today", body = TodayResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_today(State(state): State<LunchState>, user: CurrentUser) -> Result<Json<TodayResponse>, AppError> {
    let date = domain::today();
    let places = repository::list_places(&state.db, user.id).await?;
    let intentions = repository::intentions_on(&state.db, date).await?;

    let user_ids: Vec<i32> = intentions.iter().map(|(user_id, _)| *user_id).collect();
    let names = state.members.names(&user_ids).await.map_err(|e| AppError::Internal(e.to_string()))?;

    let my_place_id = intentions.iter().find(|(u, _)| *u == user.id).map(|(_, p)| *p);
    let cuisines: BTreeSet<String> =
        places.iter().filter(|p| !p.is_gamelle && !p.cuisine.is_empty()).map(|p| p.cuisine.clone()).collect();

    let places = places
        .into_iter()
        .map(|p| {
            let goers = intentions
                .iter()
                .filter(|(_, place_id)| *place_id == p.id)
                .filter_map(|(user_id, _)| {
                    names.get(user_id).map(|n| Goer {
                        user_id: *user_id,
                        first_name: n.first_name.clone(),
                        last_initial: domain::last_initial(&n.last_name),
                    })
                })
                .collect();
            PlaceResponse {
                open_today: domain::is_open_on(p.open_days, date),
                open_days: domain::mask_to_days(p.open_days),
                id: p.id,
                name: p.name,
                cuisine: p.cuisine,
                address: p.address,
                maps_url: p.maps_url,
                under_ten_euros: p.under_ten_euros,
                veggie_friendly: p.veggie_friendly,
                phone: p.phone,
                description: p.description,
                is_gamelle: p.is_gamelle,
                average_rating: p.average_rating,
                rating_count: p.rating_count,
                my_rating: p.my_rating,
                goers,
            }
        })
        .collect();

    Ok(Json(TodayResponse {
        date,
        weekday: Weekday::of(date),
        places,
        cuisines: cuisines.into_iter().collect(),
        my_place_id,
    }))
}
