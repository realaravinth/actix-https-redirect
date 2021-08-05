/*
 * Copyright (C) 2021  Aravinth Manivannan <realaravinth@batsense.net>
 *
 * Use of this source code is governed by the Apache 2.0 and/or the MIT
 * License.
 */

use crate::*;

use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::mpsc;
use std::thread;
//use std::time::Duration;

use actix_web::dev::Server;
use actix_web::{App, HttpResponse, HttpServer, Responder};
use reqwest::{redirect, Certificate, Client};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};

const PORT: usize = 9000;
const PORT_TLS: usize = 9443;
const HOST: &str = "localhost";
const KEY: &str = "key.pem";
const CERT: &str = "cert.pem";

macro_rules! get_server {
    () => {
        HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::NormalizePath::new(
                    actix_web::middleware::TrailingSlash::Trim,
                ))
                .wrap(HTTPSRedirect::new(true))
                .configure(services)
        })
    };
}

async fn run_app(tx: mpsc::Sender<Server>) -> std::io::Result<()> {
    let srv = get_server!().bind(format!("{}:{}", HOST, PORT))?.run();
    // send server controller to main thread
    let _ = tx.send(srv.clone());

    // run future
    srv.await
}

async fn run_app_tls(tx: mpsc::Sender<Server>) -> std::io::Result<()> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = rsa_private_keys(key_file).unwrap();
    config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    let srv = get_server!()
        .bind_rustls(format!("{}:{}", HOST, PORT_TLS), config)?
        .run();
    // send server controller to main thread
    let _ = tx.send(srv.clone());

    // run future
    srv.await
}

fn services(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(foo);
}

#[actix_web::get("/")]
async fn foo() -> impl Responder {
    HttpResponse::Ok()
}

fn spawn_servers() -> Vec<Server> {
    let (tx, rx) = mpsc::channel();

    let mut srv = Vec::with_capacity(2);

    thread::spawn(|| {
        actix_rt::System::new().block_on(run_app(tx)).unwrap();
    });
    srv.push(rx.recv().unwrap());
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        actix_rt::System::new().block_on(run_app_tls(tx)).unwrap();
    });

    srv.push(rx.recv().unwrap());
    srv
}

fn get_clinet() -> Client {
    let mut buf = Vec::new();
    File::open(CERT).unwrap().read_to_end(&mut buf).unwrap();
    let cert = Certificate::from_pem(&buf).unwrap();
    Client::builder()
        .use_rustls_tls()
        .add_root_certificate(cert)
        .redirect(redirect::Policy::none())
        .build()
        .unwrap()
}

#[actix_rt::test]
async fn test() {
    let srv = spawn_servers();

    let client = get_clinet();

    let redirect_req = client
        .get(format!("http://{}:{}/", HOST, PORT))
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_req.status(), StatusCode::FOUND);

    // stop servers
    for s in srv.iter() {
        s.stop(true).await;
    }

    unimplemented!();
}
