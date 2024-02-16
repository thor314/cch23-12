#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(non_snake_case)]
#![allow(clippy::clone_on_copy)]

mod error;
#[cfg(test)] mod tests;
mod utils;

use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
  time::Instant,
};

use axum::{
  extract::{Path, State},
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  Json, Router,
};
use chrono::{DateTime, Datelike, Utc, Weekday};
use error::MyError;
use futures::future::Shared;
use serde::{Deserialize, Serialize};
use tracing::info;
use ulid::Ulid;
use uuid::Uuid;

async fn hello_world() -> &'static str { "Hello, world!" }

async fn error_handler() -> impl IntoResponse {
  (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
}

// part 1
// Create two endpoints:
/// POST /12/save/<string>: takes a string and stores it.
/// GET /12/load/<string>: takes the same string and returns the number of whole seconds elapsed
/// since the last time it was stored.
///
/// example
/// ```
/// curl -X POST http://localhost:8000/12/save/packet20231212
/// sleep 2
/// curl http://localhost:8000/12/load/packet20231212
/// echo
/// sleep 2
/// curl http://localhost:8000/12/load/packet20231212
/// echo
/// curl -X POST http://localhost:8000/12/save/packet20231212
/// curl http://localhost:8000/12/load/packet20231212
/// ```
///
/// # After ~4 seconds:
/// 2
/// 4
/// 0
type SharedState = Arc<RwLock<ElapsedState>>;
#[derive(Default, Clone)]
struct ElapsedState {
  elapsed_map: HashMap<String, Instant>,
}

async fn store_string(State(elapsed_state): State<SharedState>, Path(s): Path<String>) {
  elapsed_state.write().unwrap().elapsed_map.insert(s, Instant::now());
}

// https://docs.rs/axum/latest/axum/#sharing-state-with-handlers
async fn elapsed_time(
  State(elapsed_state): State<SharedState>,
  Path(s): Path<String>,
) -> Result<String, StatusCode> {
  elapsed_state
    .read()
    .unwrap()
    .elapsed_map
    .get(&s)
    .map(|value| value.elapsed().as_secs().to_string())
    .ok_or(StatusCode::NOT_FOUND)
}

// part 2
/// Make a POST endpoint /12/ulids that takes a JSON array of ULIDs. Convert all the ULIDs to UUIDs
/// and return a new array but in reverse order.
///
/// curl -X POST http://localhost:8000/12/ulids \
/// ```
/// -H 'Content-Type: application/json' \
/// -d '[
/// "01BJQ0E1C3Z56ABCD0E11HYX4M",
/// "01BJQ0E1C3Z56ABCD0E11HYX5N",
/// "01BJQ0E1C3Z56ABCD0E11HYX6Q",
/// "01BJQ0E1C3Z56ABCD0E11HYX7R",
/// "01BJQ0E1C3Z56ABCD0E11HYX8P"
/// ]'
/// [
/// "015cae07-0583-f94c-a5b1-a070431f7516",
/// "015cae07-0583-f94c-a5b1-a070431f74f8",
/// "015cae07-0583-f94c-a5b1-a070431f74d7",
/// "015cae07-0583-f94c-a5b1-a070431f74b5",
/// "015cae07-0583-f94c-a5b1-a070431f7494"
/// ]
/// ```
async fn ulids_to_uuids(Json(ulids): Json<Vec<String>>) -> Json<Vec<Uuid>> {
  // Convert all the ULIDs to UUIDs
  let uuids: Vec<Uuid> =
    ulids.iter().filter_map(|ulid| Ulid::from_string(ulid).map(Uuid::from).ok()).rev().collect();
  Json(uuids)
}

// task 3
//
/// How many of the ULIDs were generated on a Christmas Eve?
/// How many were generated on a <weekday>? (A number in the path between 0 (Monday) and 6 (Sunday))
/// How many were generated in the future? (has a date later than the current time)
/// How many have entropy bits where the Least Significant Bit (LSB) is 1?
///
/// ```
/// curl -X POST http://localhost:8000/12/ulids/5 \
///   -H 'Content-Type: application/json' \
///   -d '[
///     "00WEGGF0G0J5HEYXS3D7RWZGV8",
///     "76EP4G39R8JD1N8AQNYDVJBRCF",
///     "018CJ7KMG0051CDCS3B7BFJ3AK",
///     "00Y986KPG0AMGB78RD45E9109K",
///     "010451HTG0NYWMPWCEXG6AJ8F2",
///     "01HH9SJEG0KY16H81S3N1BMXM4",
///     "01HH9SJEG0P9M22Z9VGHH9C8CX",
///     "017F8YY0G0NQA16HHC2QT5JD6X",
///     "03QCPC7P003V1NND3B3QJW72QJ"
///   ]'
/// ```
///
/// ```
/// {
///   "christmas eve": 3,
///   "weekday": 1,
///   "in the future": 2,
///   "LSB is 1": 5
/// }
/// ```
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UlidsResponse {
  #[serde(rename = "christmas eve")]
  pub christmas_eve: usize,
  pub weekday:       usize,
  #[serde(rename = "in the future")]
  pub in_the_future: usize,
  #[serde(rename = "LSB is 1")]
  pub lsb_is_1:      usize,
}

async fn ulids_weekday(
  Path(w): Path<u8>,
  Json(ulids): Json<Vec<String>>,
) -> Result<Json<UlidsResponse>, StatusCode> {
  let w = Weekday::try_from(w).map_err(|_| StatusCode::BAD_REQUEST)?;
  // Convert all the ULIDs to UUIDs
  let dates: Vec<DateTime<Utc>> = ulids
    .iter()
    .filter_map(|ulid| Ulid::from_string(ulid).map(|ulid| ulid.datetime().into()).ok())
    .collect();

  Ok(Json(UlidsResponse {
    christmas_eve: dates.iter().filter(|date| date.day() == 24 && date.month() == 12).count(),
    weekday:       dates.iter().filter(|date| date.weekday() == w).count(),
    in_the_future: dates.iter().filter(|date| date > &&Utc::now()).count(),
    lsb_is_1:      ulids
      .iter()
      .map(|ulid| Ulid::from_string(ulid).unwrap())
      .filter(|ulid| ulid.0 & 1 == 1)
      .count(),
  }))
}
#[shuttle_runtime::main]
async fn main(
  #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> shuttle_axum::ShuttleAxum {
  utils::setup(&secret_store).unwrap();
  let state = Arc::new(RwLock::new(ElapsedState { elapsed_map: HashMap::new() }));

  info!("hello thor");

  let router = Router::new()
    .route("/", get(hello_world))
    .route("/12/save/:s", post(store_string))
    .route("/12/load/:s", get(elapsed_time))
    .route("/12/ulids", post(ulids_to_uuids))
    .route("/12/ulids/:w", post(ulids_weekday))
    .route("/-1/error", get(error_handler))
    .route("/-1/health", get(|| async { StatusCode::OK }))
    // oh we got state
    .with_state(state);

  Ok(router.into())
}
