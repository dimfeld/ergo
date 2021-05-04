//! Auth Types:
//! Session cookie
//! API Key

use std::rc::Rc;

use actix_identity::{Identity, RequestIdentity};
use actix_web::{
    dev::{Extensions, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest, HttpMessage,
};
use futures::{
    future::{ok, ready, LocalBoxFuture, Ready},
    ready, Future, FutureExt,
};

use super::{AuthData, Authenticated, MaybeAuthenticated};

pub struct AuthenticateService {
    auth_data: Rc<AuthData>,
}

impl AuthenticateService {
    pub fn new(auth_data: AuthData) -> Self {
        AuthenticateService {
            auth_data: Rc::new(auth_data),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthenticateService
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;

    type Error = crate::error::Error;

    type Transform = AuthenticateMiddleware<S>;

    type InitError = ();

    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticateMiddleware {
            auth_data: self.auth_data.clone(),
            service: Rc::new(service),
        }))
    }
}

struct AuthenticateMiddleware<S> {
    auth_data: Rc<AuthData>,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
where
    B: 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = crate::error::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);

        async move {
            let auth = match req.get_identity() {
                Some(id) => Some(self.auth_data.authenticate(&id, &req).await?),
                None => None,
            };

            req.extensions_mut().insert(auth);
            let res = srv.call(req).await?;

            Ok(res)
        }
        .boxed_local()
    }
}
