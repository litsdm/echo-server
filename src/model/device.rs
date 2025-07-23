use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::{SurrealId, extract_id};

use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Device {
    pub id: SurrealId,
    pub name: Option<String>,
    pub platform: Option<String>,
    pub user_id: Option<String>,
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
}

#[derive(Serialize, Deserialize, Default)]
pub struct DevicePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(alias = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl From<&NewDevice> for DevicePatch {
    fn from(device: &NewDevice) -> Self {
        DevicePatch {
            name: device.name.clone(),
            platform: device.platform.clone(),
            user_id: device.user_id.clone(),
        }
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
}
