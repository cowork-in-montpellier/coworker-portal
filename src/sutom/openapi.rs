use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        crate::sutom::domain::LetterStatus,
        crate::sutom::routes::day::DayResponse,
        crate::sutom::routes::attempt::SubmitGuessesRequest,
        crate::sutom::routes::attempt::SubmitGuessesResponse,
        crate::sutom::routes::scoreboard::ScoreboardPlayer,
        crate::sutom::routes::scoreboard::ScoreboardResponse,
        crate::sutom::routes::leaderboard::LeaderboardEntry,
        crate::sutom::routes::leaderboard::LeaderboardResponse,
        crate::sutom::routes::history::HistoryEntry,
        crate::sutom::routes::history::HistoryResponse,
    )),
    tags((name = "Sutom", description = "Daily SUTOM word game")),
)]
pub struct ApiDoc;
