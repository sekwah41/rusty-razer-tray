use std::collections::{HashMap, HashSet};

use roxmltree::Document;
use serde_json::Value;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, Proxy};

use crate::openrazer::types::{Dpi, LedId, MatrixDimensions, Rgb};
use crate::openrazer::OPENRAZER_SERVICE_NAME;

pub struct Device {
    connection: Connection,
    object_path: OwnedObjectPath,
    introspection: HashSet<String>,
    supported_features: HashSet<String>,
    supported_leds: HashMap<LedId, String>,
}

impl Device {
    pub async fn new(connection: Connection, object_path: OwnedObjectPath) -> zbus::Result<Self> {
        let introspection = Self::introspect(&connection, &object_path).await?;
        let mut device = Self {
            connection,
            object_path,
            introspection,
            supported_features: HashSet::new(),
            supported_leds: HashMap::new(),
        };
        device.setup_capabilities();
        Ok(device)
    }

    pub fn object_path(&self) -> &OwnedObjectPath {
        &self.object_path
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.supported_features.contains(feature)
    }

    pub fn supported_leds(&self) -> &HashMap<LedId, String> {
        &self.supported_leds
    }

    pub async fn get_device_image_url(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        let payload: String = proxy.call("getRazerUrls", &()).await?;
        let value: Value = serde_json::from_str(&payload)
            .map_err(|err| zbus::Error::Failure(err.to_string()))?;
        Ok(value
            .get("top_img")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .to_string())
    }

    pub async fn get_device_mode(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        proxy.call("getDeviceMode", &()).await
    }

    pub async fn get_serial(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        proxy.call("getSerial", &()).await
    }

    pub async fn get_device_name(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        proxy.call("getDeviceName", &()).await
    }

    pub async fn get_device_type(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        let device_type: String = proxy.call("getDeviceType", &()).await?;
        let mapped = match device_type.as_str() {
            "core" => "accessory",
            "mousemat" => "mousepad",
            "mug" => "accessory",
            _ => device_type.as_str(),
        };
        Ok(mapped.to_string())
    }

    pub async fn get_firmware_version(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        proxy.call("getFirmware", &()).await
    }

    pub async fn get_keyboard_layout(&self) -> zbus::Result<String> {
        let proxy = self.device_misc_proxy().await?;
        let layout: String = proxy.call("getKeyboardLayout", &()).await?;
        let mapped = match layout.as_str() {
            "de_DE" => "German",
            "el_GR" => "Greek",
            "en_GB" => "UK",
            "en_US" => "US",
            "en_US_mac" => "US-mac",
            "es_ES" => "Spanish",
            "fr_FR" => "French",
            "it_IT" => "Italian",
            "ja_JP" => "Japanese",
            "pt_PT" => "Portuguese",
            _ => layout.as_str(),
        };
        Ok(mapped.to_string())
    }

    pub async fn get_poll_rate(&self) -> zbus::Result<u16> {
        let proxy = self.device_misc_proxy().await?;
        let value: i32 = proxy.call("getPollRate", &()).await?;
        Ok(value as u16)
    }

    pub async fn set_poll_rate(&self, poll_rate: u16) -> zbus::Result<()> {
        let proxy = self.device_misc_proxy().await?;
        proxy.call::<_, _, ()>("setPollRate", &(poll_rate)).await?;
        Ok(())
    }

    pub async fn get_supported_poll_rates(&self) -> zbus::Result<Vec<u16>> {
        if !self.has_capability_internal("razer.device.misc", Some("getSupportedPollRates")) {
            return Ok(vec![125, 500, 1000]);
        }
        let proxy = self.device_misc_proxy().await?;
        let values: Vec<i32> = proxy.call("getSupportedPollRates", &()).await?;
        Ok(values.into_iter().map(|value| value as u16).collect())
    }

    pub async fn set_dpi(&self, dpi: Dpi) -> zbus::Result<()> {
        let proxy = self.device_dpi_proxy().await?;
        proxy.call::<_, _, ()>("setDPI", &(dpi.dpi_x, dpi.dpi_y)).await?;
        Ok(())
    }

    pub async fn get_dpi(&self) -> zbus::Result<Dpi> {
        let proxy = self.device_dpi_proxy().await?;
        let dpi: Vec<i32> = proxy.call("getDPI", &()).await?;
        match dpi.len() {
            1 => Ok(Dpi {
                dpi_x: dpi[0] as u16,
                dpi_y: 0,
            }),
            2 => Ok(Dpi {
                dpi_x: dpi[0] as u16,
                dpi_y: dpi[1] as u16,
            }),
            _ => Err(zbus::Error::Failure(
                "Invalid return array from DPI".to_string(),
            )),
        }
    }

    pub async fn set_dpi_stages(&self, active_stage: u8, dpi_stages: Vec<Dpi>) -> zbus::Result<()> {
        let proxy = self.device_dpi_proxy().await?;
        proxy
            .call::<_, _, ()>("setDPIStages", &(active_stage, dpi_stages))
            .await?;
        Ok(())
    }

    pub async fn get_dpi_stages(&self) -> zbus::Result<(u8, Vec<Dpi>)> {
        let proxy = self.device_dpi_proxy().await?;
        proxy.call("getDPIStages", &()).await
    }

    pub async fn max_dpi(&self) -> zbus::Result<u16> {
        let proxy = self.device_dpi_proxy().await?;
        let value: i32 = proxy.call("maxDPI", &()).await?;
        Ok(value as u16)
    }

    pub async fn get_battery_percent(&self) -> zbus::Result<f64> {
        let proxy = self.device_power_proxy().await?;
        proxy.call("getBattery", &()).await
    }

    pub async fn is_charging(&self) -> zbus::Result<bool> {
        let proxy = self.device_power_proxy().await?;
        proxy.call("isCharging", &()).await
    }

    pub async fn get_allowed_dpi(&self) -> zbus::Result<Vec<u16>> {
        let proxy = self.device_dpi_proxy().await?;
        let values: Vec<i32> = proxy.call("availableDPI", &()).await?;
        if values.is_empty() {
            return Err(zbus::Error::Failure(
                "Invalid return array from availableDPI".to_string(),
            ));
        }
        Ok(values.into_iter().map(|value| value as u16).collect())
    }

    pub async fn get_idle_time(&self) -> zbus::Result<u16> {
        let proxy = self.device_power_proxy().await?;
        let value: u16 = proxy.call("getIdleTime", &()).await?;
        Ok(value)
    }

    pub async fn set_idle_time(&self, idle_time: u16) -> zbus::Result<()> {
        let proxy = self.device_power_proxy().await?;
        proxy.call::<_, _, ()>("setIdleTime", &(idle_time)).await?;
        Ok(())
    }

    pub async fn get_low_battery_threshold(&self) -> zbus::Result<u8> {
        let proxy = self.device_power_proxy().await?;
        proxy.call("getLowBatteryThreshold", &()).await
    }

    pub async fn set_low_battery_threshold(&self, threshold: f64) -> zbus::Result<()> {
        let proxy = self.device_power_proxy().await?;
        proxy.call::<_, _, ()>("setLowBatteryThreshold", &(threshold)).await?;
        Ok(())
    }

    pub async fn display_custom_frame(&self) -> zbus::Result<()> {
        let proxy = self.device_lighting_chroma_proxy().await?;
        proxy.call::<_, _, ()>("setCustom", &()).await?;
        Ok(())
    }

    pub async fn define_custom_frame(
        &self,
        row: u8,
        start_column: u8,
        end_column: u8,
        color_data: Vec<Rgb>,
    ) -> zbus::Result<()> {
        let mut data = Vec::with_capacity(3 + color_data.len() * 3);
        data.push(row);
        data.push(start_column);
        data.push(end_column);
        for color in color_data {
            data.push(color.r);
            data.push(color.g);
            data.push(color.b);
        }
        let proxy = self.device_lighting_chroma_proxy().await?;
        proxy.call::<_, _, ()>("setKeyRow", &(data)).await?;
        Ok(())
    }

    pub async fn get_matrix_dimensions(&self) -> zbus::Result<MatrixDimensions> {
        let proxy = self.device_misc_proxy().await?;
        let dims: Vec<i32> = proxy.call("getMatrixDimensions", &()).await?;
        if dims.len() != 2 {
            return Err(zbus::Error::Failure(
                "Invalid return array from getMatrixDimensions".to_string(),
            ));
        }
        Ok(MatrixDimensions {
            rows: dims[0] as u8,
            columns: dims[1] as u8,
        })
    }

    async fn introspect(
        connection: &Connection,
        object_path: &OwnedObjectPath,
    ) -> zbus::Result<HashSet<String>> {
        let proxy = Proxy::new(
            connection,
            OPENRAZER_SERVICE_NAME,
            object_path.as_str(),
            "org.freedesktop.DBus.Introspectable",
        )
        .await?;
        let xml: String = proxy.call("Introspect", &()).await?;
        let sanitized = Self::strip_doctype(&xml);
        let doc = Document::parse(&sanitized);

        let doc = match doc {
            Ok(doc) => doc,
            Err(err) => {
                return Err(zbus::Error::Failure(format!(
                    "Failed to parse introspection XML: {}",
                    err
                )))
            }
        };

        let mut entries = HashSet::new();
        for iface in doc.descendants().filter(|node| node.has_tag_name("interface")) {
            let iface_name = match iface.attribute("name") {
                Some(name) if !name.is_empty() => name,
                _ => continue,
            };
            entries.insert(iface_name.to_string());
            for method in iface.children().filter(|node| node.has_tag_name("method")) {
                if let Some(method_name) = method.attribute("name") {
                    entries.insert(format!("{iface_name};{method_name}"));
                }
            }
        }
        Ok(entries)
    }

    fn strip_doctype(xml: &str) -> String {
        let mut out = String::with_capacity(xml.len());
        let mut in_doctype = false;
        for line in xml.lines() {
            let trimmed = line.trim_start();
            if in_doctype {
                if trimmed.contains('>') {
                    in_doctype = false;
                }
                continue;
            }
            if trimmed.starts_with("<!DOCTYPE") {
                in_doctype = !trimmed.contains('>');
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    fn setup_capabilities(&mut self) {
        if self.has_capability_internal("razer.device.misc", Some("getKeyboardLayout")) {
            self.supported_features.insert("keyboard_layout".to_string());
        }
        if self.has_capability_internal("razer.device.dpi", Some("setDPI")) {
            self.supported_features.insert("dpi".to_string());
        }
        if self.has_capability_internal("razer.device.dpi", Some("availableDPI")) {
            self.supported_features
                .insert("restricted_dpi".to_string());
        }
        if self.has_capability_internal("razer.device.dpi", Some("setDPIStages")) {
            self.supported_features.insert("dpi_stages".to_string());
        }
        if self.has_capability_internal("razer.device.misc", Some("setPollRate")) {
            self.supported_features.insert("poll_rate".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.chroma", Some("setCustom")) {
            self.supported_features.insert("custom_frame".to_string());
        }
        if self.has_capability_internal("razer.device.power", Some("getBattery")) {
            self.supported_features.insert("battery".to_string());
        }
        if self.has_capability_internal("razer.device.power", Some("getLowBatteryThreshold")) {
            self.supported_features
                .insert("low_battery_threshold".to_string());
        }
        if self.has_capability_internal("razer.device.power", Some("getIdleTime")) {
            self.supported_features.insert("idle_time".to_string());
        }

        if self.has_capability_internal("razer.device.lighting.chroma", Some("setNone"))
            || self.has_capability_internal("razer.device.lighting.chroma", Some("setStatic"))
            || self.has_capability_internal("razer.device.lighting.bw2013", None)
            || self.has_capability_internal("razer.device.lighting.brightness", None)
        {
            self.supported_leds
                .insert(LedId::Unspecified, "Chroma".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.logo", None) {
            self.supported_leds
                .insert(LedId::LogoLED, "Logo".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.scroll", None) {
            self.supported_leds
                .insert(LedId::ScrollWheelLED, "Scroll".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.backlight", None) {
            self.supported_leds
                .insert(LedId::BacklightLED, "Backlight".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.left", None) {
            self.supported_leds
                .insert(LedId::LeftSideLED, "Left".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.right", None) {
            self.supported_leds
                .insert(LedId::RightSideLED, "Right".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.profile_led", Some("setRedLED")) {
            self.supported_leds
                .insert(LedId::KeymapRedLED, "RedLED".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.profile_led", Some("setGreenLED")) {
            self.supported_leds
                .insert(LedId::KeymapGreenLED, "GreenLED".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.profile_led", Some("setBlueLED")) {
            self.supported_leds
                .insert(LedId::KeymapBlueLED, "BlueLED".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.charging", None) {
            self.supported_leds
                .insert(LedId::ChargingLED, "Charging".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.fast_charging", None) {
            self.supported_leds
                .insert(LedId::FastChargingLED, "FastCharging".to_string());
        }
        if self.has_capability_internal("razer.device.lighting.fully_charged", None) {
            self.supported_leds
                .insert(LedId::FullyChargedLED, "FullyCharged".to_string());
        }
    }

    fn has_capability_internal(&self, interface: &str, method: Option<&str>) -> bool {
        match method {
            Some(method) => self
                .introspection
                .contains(&format!("{interface};{method}")),
            None => self.introspection.contains(interface),
        }
    }

    async fn device_misc_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            self.object_path.as_str(),
            "razer.device.misc",
        )
        .await
    }

    async fn device_dpi_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            self.object_path.as_str(),
            "razer.device.dpi",
        )
        .await
    }

    async fn device_power_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            self.object_path.as_str(),
            "razer.device.power",
        )
        .await
    }

    async fn device_lighting_chroma_proxy(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            OPENRAZER_SERVICE_NAME,
            self.object_path.as_str(),
            "razer.device.lighting.chroma",
        )
        .await
    }
}
