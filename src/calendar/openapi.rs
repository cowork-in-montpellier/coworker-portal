use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        crate::calendar::routes::rooms::RoomResponse,
        crate::calendar::routes::bookings::BookingResponse,
        crate::calendar::routes::bookings::CreateBookingRequest,
    )),
    tags(
        (name = "Rooms", description = "Room availability and calendar feeds"),
        (name = "Bookings", description = "Room booking management"),
    )
)]
pub struct ApiDoc;
