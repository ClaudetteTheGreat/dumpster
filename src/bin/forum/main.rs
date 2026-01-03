use actix::Actor;
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::{Key, SameSite};
use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::middleware::{DefaultHeaders, ErrorHandlers, Logger};
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use env_logger::Env;
use rand::{distributions::Alphanumeric, Rng};
use ruforo::config::create_config;
use ruforo::db::{get_db_pool, init_db};
use ruforo::middleware::ClientCtx;
use std::sync::Arc;
use std::time::Duration;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_lib_mods();
    init_our_mods();
    init_db(std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.")).await;

    // Load configuration from database
    let config = create_config();
    config
        .load_from_database(get_db_pool())
        .await
        .expect("Failed to load configuration from database");

    // Initialize rate limits from database settings
    ruforo::rate_limit::init_rate_limits(&config);

    // Initialize word filters from database
    ruforo::word_filter::init_filters(get_db_pool())
        .await
        .expect("Failed to load word filters from database");

    // Load themes into cache
    ruforo::theme::load_themes()
        .await
        .expect("Failed to load themes from database");

    let permissions = ruforo::permission::new()
        .await
        .expect("Permission System failed to initialize.");

    // Initialize global permission data store for cache invalidation
    ruforo::permission::init_permission_data(permissions.clone());

    let secret_key = match std::env::var("SECRET_KEY") {
        Ok(key) => Key::from(key.as_bytes()),
        Err(err) => {
            let random_string: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(128)
                .map(char::from)
                .collect();
            log::warn!("SECRET_KEY was invalid. Reason: {:?}\r\nThis means the key used for signing session cookies will invalidate every time the application is restarted. A secret key must be at least 64 bytes to be accepted.\r\n\r\nNeed a key? How about:\r\n{}", err, random_string);
            Key::from(random_string.as_bytes())
        }
    };

    let layer = Arc::new(ruforo::web::chat::implement::default::Layer {
        db: get_db_pool().to_owned(),
        config: config.clone(),
    });
    let chat = ruforo::web::chat::server::ChatServer::new(layer.clone(), config.clone())
        .await
        .start();

    // Start notification WebSocket server
    let notification_server = ruforo::web::notifications_ws::NotificationServer::new().start();
    ruforo::web::notifications_ws::init_notification_server(notification_server.clone());

    // Spawn rate limiter cleanup task
    actix_web::rt::spawn(async {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            ruforo::rate_limit::cleanup_old_entries_public();
            ruforo::user::cleanup_activity_cache();
            log::debug!("Rate limiter and activity cache cleanup completed");
        }
    });

    HttpServer::new(move || {
        let layer_data: Data<Arc<dyn ruforo::web::chat::implement::ChatLayer>> =
            Data::new(layer.clone());

        // Order of middleware IS IMPORTANT and is in REVERSE EXECUTION ORDER.
        // However, services are read top->down, higher traffic routes should be
        // placed higher
        App::new()
            .app_data(Data::new(get_db_pool()))
            .app_data(Data::new(permissions.clone()))
            .app_data(Data::new(config.clone()))
            .app_data(layer_data)
            .app_data(chat.clone())
            .app_data(Data::new(notification_server.clone()))
            // Security headers - applied to all responses
            .wrap(
                DefaultHeaders::new()
                    .add((header::X_FRAME_OPTIONS, "DENY"))
                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                    .add(("X-XSS-Protection", "0")) // Disable legacy XSS filter
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add((
                        "Permissions-Policy",
                        "geolocation=(), microphone=(), camera=()",
                    )),
            )
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::BAD_REQUEST, ruforo::web::error::render_400)
                    .handler(StatusCode::NOT_FOUND, ruforo::web::error::render_404)
                    .handler(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ruforo::web::error::render_500,
                    ),
            )
            .wrap(ClientCtx::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_same_site(SameSite::Lax)
                    .cookie_secure(false) // Allow HTTP for development
                    .session_lifecycle(PersistentSession::default())
                    .build(),
            )
            .wrap(Logger::new("%a %{User-Agent}i"))
            .configure(ruforo::web::configure)
    })
    // https://www.restapitutorial.com/lessons/httpmethods.html
    // GET    edit_ (get edit form)
    // PATCH  update_ (apply edit)
    // GET    view_ (read/view/render entity)
    // Note: PUT and PATCH were added, removed, and re-added(?) to the HTML5 spec for <form method="">
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

/// Initialize third party crates we rely on but don't have control over.
pub fn init_lib_mods() {
    // This should be calls to crates without any transformative work applied.
    dotenv::dotenv().expect("DotEnv failed to initialize.");
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    ffmpeg_next::init().expect("FFMPEG failed to initialize.");
}

/// Initialize all local mods.
/// Panics
pub fn init_our_mods() {
    // This should be a list of simple function calls.
    // Each module should work mostly independent of others.
    // This way, we can unit test individual modules without loading the entire application.
    ruforo::app_config::init();
    ruforo::global::init();
    ruforo::session::init();
    ruforo::filesystem::init();
}
