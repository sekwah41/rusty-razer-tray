use serde_json::Value;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;
use zbus::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

use crate::openrazer::{Device, OPENRAZER_ROOT_PATH, OPENRAZER_SERVICE_NAME};

pub struct Manager {
    connection: Connection,
}

impl Manager {
    pub async fn new() -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub async fn is_daemon_running(&self) -> zbus::Result<bool> {
        let proxy = DBusProxy::new(&self.connection).await?;
        let name = BusName::try_from(OPENRAZER_SERVICE_NAME)
            .map_err(|err| zbus::Error::Failure(err.to_string()))?;
        proxy.name_has_owner(name).await.map_err(Into::into)
    }

    pub async fn get_supported_devices(&self) -> zbus::Result<Value> {
        let proxy = self.devices_proxy().await?;
        let payload: String = proxy.call("supportedDevices", &()).await?;
        let value = serde_json::from_str(&payload)
            .map_err(|err| zbus::Error::Failure(err.to_string()))?;
        Ok(value)
    }

    pub async fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>> {
        let proxy = self.devices_proxy().await?;
        let serials: Vec<String> = proxy.call("getDevices", &()).await?;
        let mut out = Vec::with_capacity(serials.len());
        for serial in serials {
            let path = format!("/org/razer/device/{serial}");
            let object_path = OwnedObjectPath::try_from(path)
                .map_err(|err| zbus::Error::Failure(err.to_string()))?;
            out.push(object_path);
        }
        Ok(out)
    }

    pub async fn get_device(&self, object_path: OwnedObjectPath) -> zbus::Result<Device> {
        Device::new(self.connection.clone(), object_path).await
    }

    pub async fn sync_effects(&self, yes: bool) -> zbus::Result<()> {
        let proxy = self.devices_proxy().await?;
        proxy.call::<_, _, ()>("syncEffects", &(yes)).await?;
        Ok(())
    }

    pub async fn get_sync_effects(&self) -> zbus::Result<bool> {
        let proxy = self.devices_proxy().await?;
        proxy.call("getSyncEffects", &()).await
    }

    pub async fn get_daemon_version(&self) -> zbus::Result<String> {
        let proxy = self.daemon_proxy().await?;
        proxy.call("version", &()).await
    }

    pub async fn set_turn_off_on_screensaver(&self, turn_off: bool) -> zbus::Result<()> {
        let proxy = self.devices_proxy().await?;
        proxy.call::<_, _, ()>("enableTurnOffOnScreensaver", &(turn_off)).await?;
        Ok(())
    }

    pub async fn get_turn_off_on_screensaver(&self) -> zbus::Result<bool> {
        let proxy = self.devices_proxy().await?;
        proxy.call("getOffOnScreensaver", &()).await
    }

    async fn daemon_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            OPENRAZER_ROOT_PATH,
            "razer.daemon",
        )
        .await
    }

    async fn devices_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            OPENRAZER_ROOT_PATH,
            "razer.devices",
        )
        .await
    }
}
