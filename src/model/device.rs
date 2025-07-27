use std::str::FromStr;

use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::{SurrealId, extract_id};

use crate::{
    error::{Error, Result},
    model::user::User,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Device {
    pub id: SurrealId,
    pub name: Option<String>,
    pub platform: Option<String>,
    pub user_id: Option<String>,
    pub guest_id: Option<String>,
    pub expo_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewDevice {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(alias = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(alias = "guestId", skip_serializing_if = "Option::is_none")]
    pub guest_id: Option<String>,
    #[serde(alias = "expoToken", skip_serializing_if = "Option::is_none")]
    pub expo_token: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct DevicePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(alias = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(alias = "guestId", skip_serializing_if = "Option::is_none")]
    pub guest_id: Option<String>,
    #[serde(alias = "expoToken", skip_serializing_if = "Option::is_none")]
    pub expo_token: Option<String>,
}

impl From<&NewDevice> for DevicePatch {
    fn from(device: &NewDevice) -> Self {
        DevicePatch {
            name: device.name.clone(),
            platform: device.platform.clone(),
            user_id: device.user_id.clone(),
            guest_id: device.guest_id.clone(),
            expo_token: device.expo_token.clone(),
        }
    }
}

impl Device {
    pub async fn get_guest(&self, client: &Surreal<Client>) -> Result<Option<User>> {
        let guest_id = self.guest_id.clone();
        if guest_id.is_none() {
            return Ok(None);
        }

        let id = SurrealId::from_str(&guest_id.unwrap())?;
        let mut results = client
            .query("SELECT * FROM user WHERE id = $id AND user_type = 'Guest'")
            .bind(("id", id.0))
            .await?;

        let user: Option<User> = results.take(0)?;
        Ok(user)
    }
}

pub struct DeviceController;

impl DeviceController {
    pub async fn get(client: &Surreal<Client>, id: &str) -> Result<Option<Device>> {
        let device_id = extract_id(id, "device");

        let device: Option<Device> = client.select(("device", device_id)).await?;
        Ok(device)
    }

    pub async fn create_or_update(
        client: &Surreal<Client>,
        new_device: &NewDevice,
    ) -> Result<Device> {
        let stored_device = Self::get(client, &new_device.id).await?;

        let device: Device = match stored_device {
            None => {
                let device: Option<Device> = client
                    .create("device")
                    .content(new_device.to_owned())
                    .await?;
                device.ok_or(Error::StoreData("device".to_string()))?
            }
            Some(dev) => {
                let device_id = extract_id(&dev.id.to_string(), "device");
                let patch: DevicePatch = new_device.into();
                let device: Option<Device> =
                    client.update(("device", device_id)).merge(patch).await?;

                device.ok_or(Error::StoreData("device".to_string()))?
            }
        };

        Ok(device)
    }

    pub async fn update(
        client: &Surreal<Client>,
        id: &str,
        device_patch: &DevicePatch,
    ) -> Result<Device> {
        let device_id = extract_id(id, "device");

        let params = serde_json::to_value(device_patch)?;

        let device_opt: Option<Device> = client.update(("device", device_id)).merge(params).await?;
        device_opt.ok_or(Error::StoreData("device".to_string()))
    }
}
