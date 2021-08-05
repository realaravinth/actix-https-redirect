/*
 * Copyright (C) 2021  Aravinth Manivannan <realaravinth@batsense.net>
 *
 * Use of this source code is governed by the Apache 2.0 and/or the MIT
 * License.
 */
#![allow(clippy::type_complexity)]

#[cfg(test)]
mod test;

use actix_http::body::AnyBody;
use actix_http::http::{
    self,
    uri::{Scheme, Uri},
};
use actix_http::StatusCode;
use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, HttpResponseBuilder};

use futures::future::{ok, Either, Ready};

/// HTTPS Middleware: Redirect HTTP requests to HTTPs
/// Set Self.redirect to true to enable redirection.
/// In the case where it is set to false, requests are passed on to the
/// registered service unmodified
#[derive(Clone, Debug)]
pub struct HTTPSRedirect {
    pub redirect: bool,
}

impl HTTPSRedirect {
    /// create new instance of of HTTPS redirect middleware
    pub fn new(redirect: bool) -> Self {
        Self { redirect }
    }
}

impl<S> Transform<S, ServiceRequest> for HTTPSRedirect
where
    S: Service<ServiceRequest, Response = ServiceResponse<AnyBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<AnyBody>;
    type Error = Error;
    type Transform = HTTPSRedirectMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HTTPSRedirectMiddleware {
            service,
            redirect: self.redirect,
        })
    }
}
pub struct HTTPSRedirectMiddleware<S> {
    service: S,
    redirect: bool,
}

impl<S> Service<ServiceRequest> for HTTPSRedirectMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<AnyBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<AnyBody>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let (r, pl) = req.into_parts();

        let mut uri_parts = r.uri().clone().into_parts();

        if self.redirect && uri_parts.scheme == Some(Scheme::HTTP) {
            let req = ServiceRequest::from_parts(r, pl); //.ok().unwrap();
            uri_parts.scheme = Some(Scheme::HTTP);
            let uri = Uri::from_parts(uri_parts).unwrap();
            Either::Right(ok(req.into_response(
                HttpResponseBuilder::new(StatusCode::FOUND)
                    .append_header((http::header::LOCATION, uri.to_string()))
                    .finish(),
            )))
        } else {
            let req = ServiceRequest::from_parts(r, pl); //.ok().unwrap();
            Either::Left(self.service.call(req))
        }
    }
}
