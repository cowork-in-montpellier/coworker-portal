use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        crate::lunch::domain::Weekday,
        crate::lunch::domain::PlaceInput,
        crate::lunch::routes::today::Goer,
        crate::lunch::routes::today::PlaceResponse,
        crate::lunch::routes::today::TodayResponse,
        crate::lunch::routes::places::PlaceIdResponse,
        crate::lunch::routes::rating::SetRatingRequest,
        crate::lunch::routes::rating::SetRatingResponse,
        crate::lunch::routes::intention::SetIntentionRequest,
        crate::lunch::routes::intention::SetIntentionResponse,
    )),
    tags((name = "Lunch", description = "Pause-déj: where coworkers have lunch")),
)]
pub struct ApiDoc;
