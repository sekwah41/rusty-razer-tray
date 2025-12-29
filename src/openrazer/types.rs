use serde::{Deserialize, Serialize};
use zvariant::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct Dpi {
    pub dpi_x: u16,
    pub dpi_y: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct MatrixDimensions {
    pub rows: u8,
    pub columns: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedId {
    Unspecified,
    LogoLED,
    ScrollWheelLED,
    BacklightLED,
    LeftSideLED,
    RightSideLED,
    KeymapRedLED,
    KeymapGreenLED,
    KeymapBlueLED,
    ChargingLED,
    FastChargingLED,
    FullyChargedLED,
}
