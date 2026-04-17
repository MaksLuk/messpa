use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(),
    components(schemas()),
    info(
        title = "Messpa API",
        description = "Messpa API Docs",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;
