use actix_web::{middleware, App, HttpServer};
use db::DbHandle;

// Copyright (C) 2021 RoccoDev
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::io;
use std::sync::Arc;

mod db;
mod error;
mod modes;
mod protos;
mod web;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbHandle>,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let port = std::env::var("KIG_PORT").unwrap_or_else(|_| "3233".to_string());
    let host = std::env::var("KIG_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let db = Arc::new(DbHandle::new().await.unwrap());
    let state = AppState { db };

    println!("Server running at http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(actix_web::web::Data::new(state.clone()))
            .service(web::static_files())
            .service(web::static_files_fallback())
            .service(web::images())
            .service(web::images_fallback())
            .service(web::add_routes())
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}