use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(crate::sutom::routes::today::TodayResponse)),
    tags((name = "Sutom", description = "Daily SUTOM word game")),
)]
pub struct ApiDoc;
