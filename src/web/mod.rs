pub mod account;
pub mod activity;
pub mod admin;
pub mod asset;
pub mod chat;
pub mod conversations;
pub mod email_verification;
pub mod error;
pub mod feed;
pub mod forum;
pub mod index;
pub mod login;
pub mod logout;
pub mod member;
pub mod notifications;
pub mod notifications_ws;
pub mod password_reset;
pub mod polls;
pub mod post;
pub mod reactions;
pub mod recent;
pub mod reports;
pub mod search;
pub mod thread;
pub mod unfurl;

/// Configures the web app by adding services from each web file.
///
/// @see https://docs.rs/actix-web/4.0.1/actix_web/struct.App.html#method.configure
pub fn configure(conf: &mut actix_web::web::ServiceConfig) {
    // Descending order. Order is important.
    // Route resolution will stop at the first match.
    index::configure(conf);
    account::configure(conf);
    activity::configure(conf);
    admin::configure(conf);
    asset::configure(conf);
    chat::configure(conf);
    conversations::configure(conf);
    email_verification::configure(conf);
    feed::configure(conf);
    forum::configure(conf);
    login::configure(conf);
    logout::configure(conf);
    member::configure(conf);
    notifications::configure(conf);
    notifications_ws::configure(conf);
    password_reset::configure(conf);
    polls::configure(conf);
    post::configure(conf);
    reactions::configure(conf);
    recent::configure(conf);
    reports::configure(conf);
    search::configure(conf);
    thread::configure(conf);
    unfurl::configure(conf);

    conf.service(crate::create_user::create_user_get)
        .service(crate::create_user::create_user_post)
        .service(crate::auth_2fa::user_enable_2fa)
        .service(crate::filesystem::post_file_hash)
        .service(crate::filesystem::put_file)
        .service(crate::session::view_task_expire_sessions);
}
