use axum::body::Body;
use axum::http::Request;
use dotenvy::dotenv;
use some_axum_code::router::api_router;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use ulid::Ulid;

#[tokio::main]
async fn main() {
    dotenv().expect(".env file should be present");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "some_axum_code=debug,tower_http=debug,axum::rejection=trace,sqlx=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_connection_str = std::env::var("DATABASE_URL").expect("DATABASE_URL should be present");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let app = api_router(pool).layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(UlidRequestId {
                ulid: Ulid::new(),
            }))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                    let request_id = request
                        .extensions()
                        .get::<RequestId>()
                        .unwrap()
                        .header_value()
                        .to_str()
                        .unwrap();

                    tracing::debug_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id
                    )
                }),
            )
            .layer(CompressionLayer::new()),
    );

    let server =
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app.into_make_service());

    tracing::debug!("listening on {}", server.local_addr());

    server.await.unwrap()
}

#[derive(Clone, Default)]
struct UlidRequestId {
    ulid: Ulid,
}

impl MakeRequestId for UlidRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = self.ulid.to_string().parse().unwrap();

        Some(RequestId::new(id))
    }
}
