use bevy::color::{Color, ColorToComponents, Srgba};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LightDefinition {
    pub color: [f32; 3],
}

impl From<[u8; 4]> for LightDefinition {
    fn from(rgba: [u8; 4]) -> Self {
        let [r, g, b, a] = rgba;
        let alpha = a as f32 / 255.0;
        LightDefinition {
            color: [
                (r as f32 / 255.0) * alpha,
                (g as f32 / 255.0) * alpha,
                (b as f32 / 255.0) * alpha,
            ],
        }
    }
}

impl From<[u8; 3]> for LightDefinition {
    fn from(rgb: [u8; 3]) -> Self {
        let [r, g, b] = rgb;
        LightDefinition {
            color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
        }
    }
}

impl From<Srgba> for LightDefinition {
    fn from(srgba: Srgba) -> Self {
        let color: Color = srgba.into(); // convert Srgba -> Color
        let [r, g, b, a] = color.to_srgba().to_f32_array(); // now extract linear values
        Self {
            color: [r * 1.0, g * 1.0, b * 1.0], // premultiplied alpha
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UndirectedLightEmitter {
    pub props: LightDefinition,
}
