use crate::db::get_db_pool;
use crate::permission::PermissionData;
use crate::user::Profile;
use actix::fut::ready;
use actix_session::Session;
use actix_web::dev::{
    self, Extensions, Payload, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::{web::Data, Error, FromRequest, HttpMessage, HttpRequest};
use futures::future::{err, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Client data stored for a single request cycle.
/// Distinct from ClientCtx because it is defined through request data.
#[derive(Clone, Debug)]
pub struct ClientCtxInner {
    /// User data. Optional. None is a guest user.
    pub client: Option<Profile>,
    /// List of user group ids. Guests may receive unregistered/portal roles.
    pub groups: Vec<i32>,
    /// Permission data.
    pub permissions: Data<PermissionData>,
    /// Randomly generated string for CSR.
    pub nonce: String,
    /// CSRF token for form protection
    pub csrf_token: String,
    /// Unread notification count for the user
    pub unread_notifications: i64,
    /// Unread message count for the user
    pub unread_messages: i64,
    /// Time the request started for page load statistics.
    pub request_start: Instant,
}

impl Default for ClientCtxInner {
    fn default() -> Self {
        Self {
            // Guests and users.
            permissions: Data::new(PermissionData::default()),
            groups: Vec::new(),
            // Only users.
            client: None,
            // Generally left default.
            nonce: Self::nonce(),
            csrf_token: String::new(), // Will be populated from session
            unread_notifications: 0,
            unread_messages: 0,
            request_start: Instant::now(),
        }
    }
}

impl ClientCtxInner {
    pub async fn from_session(session: &Session, permissions: Data<PermissionData>) -> Self {
        use crate::group::get_group_ids_for_client;
        use crate::middleware::csrf::get_or_create_csrf_token;
        use crate::session::authenticate_client_by_session;

        let db = get_db_pool();
        let client = authenticate_client_by_session(session).await;
        let groups = get_group_ids_for_client(db, &client).await;

        // Get or create CSRF token for this session
        let csrf_token = get_or_create_csrf_token(session).unwrap_or_else(|_| String::new());

        // Get unread notification count for logged-in users
        let unread_notifications = if let Some(ref user) = client {
            crate::notifications::count_unread_notifications(user.id)
                .await
                .unwrap_or(0)
        } else {
            0
        };

        // Get unread message count for logged-in users
        let unread_messages = if let Some(ref user) = client {
            crate::conversations::count_unread_conversations(user.id)
                .await
                .unwrap_or(0)
        } else {
            0
        };

        ClientCtxInner {
            client,
            groups,
            permissions,
            csrf_token,
            unread_notifications,
            unread_messages,
            ..Default::default()
        }
    }

    /// Returns a hash unique to each request used for CSP.
    /// See: <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/nonce>
    /// and <https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP>
    pub fn nonce() -> String {
        let mut hasher = blake3::Hasher::new();

        // Hash: Salt
        match std::env::var("SALT") {
            Ok(v) => hasher.update(v.as_bytes()),
            Err(_) => hasher.update("NO_SALT_FOR_NONCE".as_bytes()),
        };

        // Hash: Timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        hasher.update(
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System clock before 1970. Really?")
                .as_millis()
                .to_ne_bytes(),
        );
        hasher.finalize().to_string()
    }
}

/// Client context passed to routes.
/// Wraps ClientCtxInner, which is set at the beginning of the request.
#[derive(Clone, Debug)]
pub struct ClientCtx(Data<ClientCtxInner>);

impl Default for ClientCtx {
    fn default() -> Self {
        Self(Data::new(ClientCtxInner::default()))
    }
}

impl ClientCtx {
    /// Returns instance of Self with components required for ClientCtxInner.
    pub async fn from_session(session: &Session, permissions: Data<PermissionData>) -> Self {
        Self(Data::new(
            ClientCtxInner::from_session(session, permissions).await,
        ))
    }

    pub fn get_or_default_from_extensions(
        extensions: &mut Extensions,
        permissions: Data<PermissionData>,
    ) -> Self {
        match extensions.get::<Data<ClientCtxInner>>() {
            // Existing record in extensions; pull it and return clone.
            Some(cbox) => Self(cbox.clone()),
            // No existing record; create and insert it.
            None => {
                let cbox = Data::new(ClientCtxInner {
                    // Add permission Arc reference to our inner value.
                    permissions: permissions,
                    ..Default::default()
                });
                // Insert ClientCtx into extensions jar.
                extensions.insert(cbox.clone());
                Self(cbox)
            }
        }
    }

    pub fn get_groups(&self) -> Vec<i32> {
        self.0.groups.to_owned()
    }

    /// Returns either the user's id or None.
    pub fn get_id(&self) -> Option<i32> {
        self.0.client.as_ref().map(|u| u.id)
    }

    /// Returns either the user's name or the word for guest.
    /// TODO: l10n "Guest"
    pub fn get_name(&self) -> String {
        let user = &self.0.client;
        match user {
            Some(user) => user.name.to_owned(),
            None => "Guest".to_owned(),
        }
    }

    pub fn get_user(&self) -> Option<&Profile> {
        self.0.client.as_ref()
    }

    pub fn get_csrf_token(&self) -> &str {
        &self.0.csrf_token
    }

    pub fn get_unread_notifications(&self) -> i64 {
        self.0.unread_notifications
    }

    pub fn get_unread_messages(&self) -> i64 {
        self.0.unread_messages
    }

    pub fn is_user(&self) -> bool {
        self.0.client.is_some()
    }

    pub fn can(&self, tag: &str) -> bool {
        self.0.permissions.can(self, tag)
    }

    pub fn can_post_in_thread(&self, _thread: &crate::orm::threads::Model) -> bool {
        self.can_post_in_forum()
    }

    pub fn can_post_in_forum(&self) -> bool {
        true
    }

    pub fn can_delete_post(&self, post: &crate::web::post::PostForTemplate) -> bool {
        self.is_user() && self.get_id() == post.user_id
    }

    pub fn can_update_post(&self, post: &crate::web::post::PostForTemplate) -> bool {
        self.is_user() && self.get_id() == post.user_id
    }

    pub fn can_read_post(&self, post: &crate::web::post::PostForTemplate) -> bool {
        if post.deleted_at.is_none() {
            // Post is not deleted, everyone can see it
            return true;
        }

        // Post is deleted - check if user can view it
        if crate::constants::ALLOW_VIEW_OWN_DELETED {
            // Allow authors to see their own deleted posts
            self.get_id() == post.user_id
        } else {
            // Nobody can see deleted posts (except moderators in the future)
            false
        }
    }

    pub fn get_nonce(&self) -> &String {
        &self.0.nonce
    }

    pub fn get_permissions(&self) -> &Data<PermissionData> {
        &self.0.permissions
    }

    /// Returns Duration representing request time.
    pub fn request_time(&self) -> Duration {
        Instant::now() - self.0.request_start
    }

    /// Returns human readable representing request time.
    pub fn request_time_as_string(&self) -> String {
        let us = self.request_time().as_micros();
        if us > 5000 {
            format!("{}ms", us / 1000)
        } else {
            format!("{}μs", us)
        }
    }

    /// Require user to be logged in. Returns user_id or ErrorUnauthorized.
    pub fn require_login(&self) -> Result<i32, actix_web::Error> {
        self.get_id()
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("Login required"))
    }

    /// Require specific permission. Returns () or ErrorForbidden.
    pub fn require_permission(&self, permission: &str) -> Result<(), actix_web::Error> {
        if !self.can(permission) {
            return Err(actix_web::error::ErrorForbidden("Insufficient permissions"));
        }
        Ok(())
    }

    /// Check if user can modify content (owner or has permission).
    /// This is a more flexible version of can_delete_post/can_update_post.
    pub fn can_modify(&self, resource_user_id: Option<i32>, permission: &str) -> bool {
        // Check if user has the permission (e.g., moderator)
        if self.can(permission) {
            return true;
        }

        // Check if user owns the resource
        if let Some(user_id) = self.get_id() {
            if let Some(owner_id) = resource_user_id {
                return user_id == owner_id;
            }
        }

        false
    }

    /// Require ownership of a resource. Returns () or ErrorForbidden.
    pub fn require_ownership(&self, resource_user_id: Option<i32>) -> Result<(), actix_web::Error> {
        let user_id = self.require_login()?;

        match resource_user_id {
            Some(owner_id) if owner_id == user_id => Ok(()),
            _ => Err(actix_web::error::ErrorForbidden("You don't own this resource")),
        }
    }
}

/// This implementation is what actually provides the `client: ClientCtx` in the parameters of route functions.
impl FromRequest for ClientCtx {
    /// The associated error which can be returned.
    type Error = Error;
    /// Future that resolves to a Self.
    type Future = Ready<Result<Self, Self::Error>>;

    /// Create a Self from request parts asynchronously.
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(perm_arc) = req.app_data::<Data<PermissionData>>() {
            ready(Ok(ClientCtx::get_or_default_from_extensions(
                &mut req.extensions_mut(),
                perm_arc.clone(),
            )))
        } else {
            err(actix_web::error::ErrorServiceUnavailable(
                "Permission data is not loaded.",
            ))
        }
    }
}

impl<S: 'static, B> Transform<S, ServiceRequest> for ClientCtx
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ClientCtxMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ClientCtxMiddleware {
            service: Rc::new(service),
            inner: self.0.clone(),
        }))
    }
}

/// Client context middleware
pub struct ClientCtxMiddleware<S> {
    service: Rc<S>,
    #[allow(dead_code)]
    inner: Data<ClientCtxInner>,
}

impl<S, B> Service<ServiceRequest> for ClientCtxMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        // Borrows of `req` must be done in a precise way to avoid conflcits. This order is important.
        let (httpreq, payload) = req.into_parts();
        let session = Session::extract(&httpreq).into_inner();
        let req = ServiceRequest::from_parts(httpreq, payload);

        // If we do not have permission data there is no client interface to access.
        Box::pin(async move {
            if let Some(perm_arc) = req.app_data::<Data<PermissionData>>() {
                let perm_arc = perm_arc.clone();

                match session {
                    Ok(session) => req.extensions_mut().insert(Data::new(
                        ClientCtxInner::from_session(&session, perm_arc).await,
                    )),
                    Err(err) => {
                        log::error!("Unable to extract Session data in middleware: {}", err);
                        None
                    }
                };
            };

            svc.call(req).await
        })
    }
}
