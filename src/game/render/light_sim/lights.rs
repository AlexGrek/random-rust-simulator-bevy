#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LightDefinition {
    pub color: [i32; 3],
}

const LN_256: f64 = 5.545177444479562;

impl From<[u8; 3]> for LightDefinition {
    fn from([r, g, b]: [u8; 3]) -> Self {
        fn scale(c: u8) -> i32 {
            let scaled = (1.0 + c as f64).ln() / LN_256;
            (scaled * i32::MAX as f64).round() as i32
        }
        LightDefinition {
            color: [scale(r), scale(g), scale(b)],
        }
    }
}

impl From<[u8; 4]> for LightDefinition {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        let alpha = a as f64 / 255.0;
        fn scale(c: u8, alpha: f64) -> i32 {
            let scaled = (1.0 + c as f64).ln() / LN_256;
            (scaled * alpha * i32::MAX as f64).round() as i32
        }
        LightDefinition {
            color: [scale(r, alpha), scale(g, alpha), scale(b, alpha)],
        }
    }
}

impl LightDefinition {
    pub fn get_color_rgba(&self) -> [u8; 4] {
        let max_i = self.color.iter().copied().max().unwrap_or(0).max(1);
        let alpha = (max_i as f64 / i32::MAX as f64).clamp(0.0, 1.0);

        fn inverse_scale(c: i32, alpha: f64) -> u8 {
            if alpha == 0.0 {
                0
            } else {
                let norm = c as f64 / (i32::MAX as f64 * alpha);
                ((norm * LN_256).exp() - 1.0).clamp(0.0, 255.0).round() as u8
            }
        }

        let [r, g, b] = self.color;
        [
            inverse_scale(r, alpha),
            inverse_scale(g, alpha),
            inverse_scale(b, alpha),
            (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    pub fn set_color_rgba(&mut self, rgba: [u8; 4]) {
        *self = Self::from(rgba);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UndirectedLightEmitter {
    pub props: LightDefinition
}