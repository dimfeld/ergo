//! Auth Types:
//! Session cookie
//! API Key

use std::rc::Rc;

use actix_identity::RequestIdentity;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::{
    future::{ready, LocalBoxFuture, Ready},
    FutureExt,
};

use super::{AuthData, AuthenticationInfo};

pub struct AuthenticateMiddlewareFactory {
    auth_data: Rc<AuthData>,
}

impl AuthenticateMiddlewareFactory {
    pub fn new(auth_data: AuthData) -> Self {
        AuthenticateMiddlewareFactory {
            auth_data: Rc::new(auth_data),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthenticateMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;

    type Error = Error;

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

pub struct AuthenticateMiddleware<S> {
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
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);
        let auth_data = self.auth_data.clone();

        async move {
            let id = req.get_identity();
            let auth = auth_data.authenticate(id, &req).await?;
            if let Some(auth) = auth {
                req.extensions_mut()
                    .insert::<Rc<AuthenticationInfo>>(Rc::new(auth));
            }

            let res = srv.call(req).await?;

            Ok(res)
        }
        .boxed_local()
    }
}
