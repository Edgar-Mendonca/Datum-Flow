//! The built-in fluids: water, air and a light oil at a chosen temperature, each with the source
//! of its values. "custom" keeps whatever the user typed.

use crate::model::Fluid;

/// Water at 1 atm: (°C, density kg/m³, viscosity mPa·s, conductivity W/m·K, specific heat J/kg·K,
/// expansion 1e-4/K). Rounded handbook values of the IAPWS formulations.
const WATER: [(f64, f64, f64, f64, f64, f64); 9] = [
    (10.0, 999.7, 1.306, 0.580, 4192.0, 0.88),
    (20.0, 998.2, 1.002, 0.598, 4182.0, 2.07),
    (30.0, 995.7, 0.7977, 0.615, 4178.0, 3.03),
    (40.0, 992.2, 0.6532, 0.631, 4179.0, 3.85),
    (50.0, 988.0, 0.5470, 0.644, 4181.0, 4.57),
    (60.0, 983.2, 0.4665, 0.654, 4185.0, 5.23),
    (70.0, 977.8, 0.4040, 0.663, 4190.0, 5.84),
    (80.0, 971.8, 0.3544, 0.670, 4197.0, 6.41),
    (90.0, 965.3, 0.3145, 0.675, 4205.0, 6.96),
];

/// The temperature range (K) a preset's values cover.
pub fn range(preset: &str) -> Option<(f64, f64)> {
    match preset {
        "water" => Some((283.15, 363.15)),
        "air" => Some((223.15, 573.15)),
        "oil" => Some((273.15, 393.15)),
        _ => None,
    }
}

/// A built-in fluid at temperature `t` (K), clamped to the range its values cover.
pub fn preset(name: &str, t: f64) -> Option<Fluid> {
    let (lo, hi) = range(name)?;
    let t = if t.is_finite() { t.clamp(lo, hi) } else { 293.15 };
    let c = t - 273.15;
    Some(match name {
        "water" => {
            let i = WATER.iter().position(|r| r.0 >= c).unwrap_or(WATER.len() - 1).max(1);
            let (a, b) = (WATER[i - 1], WATER[i]);
            let s = ((c - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
            let l = |x: f64, y: f64| x + (y - x) * s;
            // Viscosity falls roughly exponentially: interpolate its logarithm.
            let mu = (l(a.2.ln(), b.2.ln())).exp() * 1e-3;
            Fluid { preset: "water".into(), name: "Water".into(), at: t, rho: l(a.1, b.1), mu, k: l(a.3, b.3), cp: l(a.4, b.4), beta: l(a.5, b.5) * 1e-4,
                source: "Liquid water at 1 atm: rounded handbook values of the IAPWS formulations (IAPWS-95; viscosity 2008; conductivity 2011), interpolated between 10 °C steps.".into() }
        }
        "air" => {
            // Dry air as an ideal gas at 1 atm; Sutherland's law for viscosity and conductivity.
            let rho = 101_325.0 / (287.05 * t);
            let mu = 1.716e-5 * (t / 273.15).powf(1.5) * (273.15 + 110.4) / (t + 110.4);
            let k = 0.0241 * (t / 273.15).powf(1.5) * (273.15 + 194.0) / (t + 194.0);
            Fluid { preset: "air".into(), name: "Air".into(), at: t, rho, mu, k, cp: 1006.0, beta: 1.0 / t,
                source: "Dry air at 1 atm: ideal gas (R = 287.05 J/kg·K, β = 1/T); viscosity and conductivity by Sutherland's law with the constants of White, Viscous Fluid Flow (2006); cp = 1006 J/kg·K.".into() }
        }
        "oil" => {
            // A light mineral oil of viscosity grade ISO VG 10: 10 mm²/s at 40 °C by definition
            // (ISO 3448), about 2.6 mm²/s at 100 °C; in between by the ASTM D341 chart equation
            // log10 log10(ν + 0.7) = A − B log10 T.
            let w = |nu: f64| (nu + 0.7).log10().log10();
            let (t1, t2) = (313.15f64, 373.15f64);
            let b = (w(10.0) - w(2.6)) / (t2.log10() - t1.log10());
            let a = w(10.0) + b * t1.log10();
            let nu = (10f64.powf(10f64.powf(a - b * t.log10())) - 0.7) * 1e-6;
            let rho = 850.0 * (1.0 - 0.00065 * (c - 15.0));
            Fluid { preset: "oil".into(), name: "Light oil (ISO VG 10)".into(), at: t, rho, mu: nu * rho, k: 0.13, cp: 1880.0 + 3.7 * (c - 20.0), beta: 0.00065,
                source: "A typical light mineral oil of grade ISO VG 10 (ISO 3448: 10 mm²/s at 40 °C); viscosity against temperature by ASTM D341; typical density, conductivity and specific heat. For a real product use Custom with its data sheet.".into() }
        }
        _ => return None,
    })
}

/// A fluid from plain numbers (benchmarks and "custom").
pub fn custom(name: &str, rho: f64, mu: f64, k: f64, cp: f64, beta: f64) -> Fluid {
    Fluid { preset: "custom".into(), name: name.into(), at: 293.15, rho, mu, k, cp, beta, source: "Values typed by the user.".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_give_the_handbook_values() {
        let w = preset("water", 293.15).unwrap();
        assert!((w.rho - 998.2).abs() < 0.05 && (w.mu - 1.002e-3).abs() < 1e-6 && (w.pr() - 7.0).abs() < 0.1, "{w:?}");
        // Between table rows, and clamped outside them.
        let w25 = preset("water", 298.15).unwrap();
        assert!((w25.mu - 0.89e-3).abs() < 0.01e-3 && (w25.rho - 997.0).abs() < 0.3, "{w25:?}");
        assert_eq!(preset("water", 1000.0).unwrap().at, 363.15);
        let a = preset("air", 293.15).unwrap();
        assert!((a.rho - 1.204).abs() < 0.002 && (a.mu - 1.81e-5).abs() < 2e-7 && (a.k - 0.0257).abs() < 5e-4 && (a.pr() - 0.71).abs() < 0.02, "{a:?}");
        let o = preset("oil", 313.15).unwrap();
        assert!((o.nu() - 10e-6).abs() < 1e-9, "{}", o.nu());
        assert!((preset("oil", 373.15).unwrap().nu() - 2.6e-6).abs() < 1e-9);
        assert!(preset("mercury", 300.0).is_none() && preset("water", f64::NAN).is_some());
    }
}
