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

use actix_files::Files;
use actix_web::{web, Scope};
use cached::proc_macro::once;
use std::time::Duration;
use time::OffsetDateTime;

mod gamelog;

pub fn add_routes() -> Scope {
    web::scope("/")
        .route("/game/{mode}/{id}", web::get().to(gamelog::gamelog_by_id))
        .route("/demo/{mode}", web::get().to(gamelog::demo_gamelog))
}

pub fn static_files() -> Files {
    Files::new("/static", "static").show_files_listing()
}

pub fn static_files_fallback() -> Files {
    Files::new("/game-static", "static").show_files_listing()
}

pub fn images() -> Files {
    Files::new("/img", "img")
}

pub fn images_fallback() -> Files {
    Files::new("/game-img", "img")
}

#[once(time = 3600)]
fn get_current_time() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}
