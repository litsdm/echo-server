use actix_web::{
    patch,
    web::{Data, Json, Path},
};

use crate::{
    error::Result,
    model::device::{Device, DeviceController, DevicePatch},
    repo::surreal::SurrealDB,
};

#[patch("/{id}")]
pub async fn update_device(
    db: Data<SurrealDB>,
    path: Path<String>,
    body: Json<DevicePatch>,
) -> Result<Json<Device>> {
    let updated_device =
        DeviceController::update(&db.surreal, &path.into_inner(), &body.into_inner()).await?;
    Ok(Json(updated_device))
}
