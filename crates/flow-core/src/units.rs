//! Display units. The engine and the file use SI base units; this table is the one place values
//! are converted for people (the window, the command line and the reports all read it).

use crate::model::Units;
use serde::Serialize;

/// The kinds of quantity Datum Flow shows.
pub const KINDS: [&str; 22] = ["len", "speed", "pressure", "temp", "dtemp", "density", "viscosity", "kinvisc", "cond", "cp", "expansion", "flux", "flow", "force", "power", "accel", "vorticity", "stream", "area", "htc", "ratio", "time"];

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Unit {
    /// Display value = SI value × factor + offset.
    pub factor: f64,
    pub offset: f64,
    pub label: &'static str,
}

const FT: f64 = 1.0 / 0.3048;
const LB: f64 = 1.0 / 0.453_592_37;
const LBF: f64 = 1.0 / 4.448_221_615_260_5;
const BTU: f64 = 1.0 / 1_055.055_852_62;

/// The unit a kind of quantity is shown in.
pub fn unit(units: Units, kind: &str) -> Unit {
    let u = |factor: f64, label: &'static str| Unit { factor, offset: 0.0, label };
    match (units, kind) {
        (Units::SI, "len") => u(1.0, "m"),
        (Units::SI, "speed") => u(1.0, "m/s"),
        (Units::SI, "pressure") => u(1.0, "Pa"),
        (Units::SI, "temp") => Unit { factor: 1.0, offset: -273.15, label: "°C" },
        (Units::SI, "dtemp") => u(1.0, "K"),
        (Units::SI, "density") => u(1.0, "kg/m³"),
        (Units::SI, "viscosity") => u(1.0, "Pa·s"),
        (Units::SI, "kinvisc") => u(1.0, "m²/s"),
        (Units::SI, "cond") => u(1.0, "W/m·K"),
        (Units::SI, "cp") => u(1.0, "J/kg·K"),
        (Units::SI, "expansion") => u(1.0, "1/K"),
        (Units::SI, "flux") => u(1.0, "W/m²"),
        (Units::SI, "flow") => u(1.0, "m³/s"),
        (Units::SI, "force") => u(1.0, "N"),
        (Units::SI, "power") => u(1.0, "W"),
        (Units::SI, "accel") => u(1.0, "m/s²"),
        (Units::SI, "stream") => u(1.0, "m²/s"),
        (Units::SI, "area") => u(1.0, "m²"),
        (Units::SI, "htc") => u(1.0, "W/m²·K"),
        (Units::US, "len") => u(FT, "ft"),
        (Units::US, "speed") => u(FT, "ft/s"),
        (Units::US, "pressure") => u(LBF / (FT * FT) / 144.0, "psi"),
        (Units::US, "temp") => Unit { factor: 1.8, offset: -459.67, label: "°F" },
        (Units::US, "dtemp") => u(1.8, "°F"),
        (Units::US, "density") => u(LB / (FT * FT * FT), "lb/ft³"),
        (Units::US, "viscosity") => u(LB / FT, "lb/ft·s"),
        (Units::US, "kinvisc") => u(FT * FT, "ft²/s"),
        (Units::US, "cond") => u(BTU * 3600.0 / (FT * 1.8), "BTU/h·ft·°F"),
        (Units::US, "cp") => u(BTU / (LB * 1.8), "BTU/lb·°F"),
        (Units::US, "expansion") => u(1.0 / 1.8, "1/°F"),
        (Units::US, "flux") => u(BTU * 3600.0 / (FT * FT), "BTU/h·ft²"),
        (Units::US, "flow") => u(FT * FT * FT, "ft³/s"),
        (Units::US, "force") => u(LBF, "lbf"),
        (Units::US, "power") => u(BTU * 3600.0, "BTU/h"),
        (Units::US, "accel") => u(FT, "ft/s²"),
        (Units::US, "stream") => u(FT * FT, "ft²/s"),
        (Units::US, "area") => u(FT * FT, "ft²"),
        (Units::US, "htc") => u(BTU * 3600.0 / (FT * FT * 1.8), "BTU/h·ft²·°F"),
        (_, "vorticity") => u(1.0, "1/s"),
        (_, "time") => u(1.0, "s"),
        _ => u(1.0, ""),
    }
}

/// SI value → display value.
pub fn to_display(units: Units, kind: &str, v: f64) -> f64 {
    let u = unit(units, kind);
    v * u.factor + u.offset
}
/// Display value → SI value.
pub fn from_display(units: Units, kind: &str, v: f64) -> f64 {
    let u = unit(units, kind);
    (v - u.offset) / u.factor
}

/// A number with `sig` significant figures, without needless zeros ("0.1175", "5.58", "1.2e-7").
pub fn num(v: f64, sig: usize) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    if !(1e-4..1e7).contains(&a) {
        let s = format!("{:.*e}", sig.saturating_sub(1), v);
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
        return format!("{m}e{e}");
    }
    let e = a.log10().floor() as i32;
    let digits = (sig as i32 - 1 - e).max(0) as usize;
    // Whole numbers longer than the figures asked for are rounded too (11954 → 12000).
    let v = if e >= sig as i32 { let p = 10f64.powi(e - sig as i32 + 1); (v / p).round() * p } else { v };
    let s = format!("{v:.digits$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

/// A value with its unit, for reports and messages.
pub fn show(units: Units, kind: &str, v: f64, sig: usize) -> String {
    let u = unit(units, kind);
    if u.label.is_empty() { num(v * u.factor + u.offset, sig) } else { format!("{} {}", num(v * u.factor + u.offset, sig), u.label) }
}

/// The whole table for one unit system, for the window.
pub fn table(units: Units) -> std::collections::BTreeMap<&'static str, Unit> {
    KINDS.iter().map(|k| (*k, unit(units, k))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversions_match_hand_values_and_round_trip() {
        // 1 atm = 14.696 psi; 20 °C = 68 °F; 1 m/s = 3.2808 ft/s; water 998.2 kg/m³ = 62.32 lb/ft³.
        assert!((to_display(Units::US, "pressure", 101_325.0) - 14.696).abs() < 1e-3);
        assert!((to_display(Units::US, "temp", 293.15) - 68.0).abs() < 1e-9);
        assert!((to_display(Units::SI, "temp", 293.15) - 20.0).abs() < 1e-9);
        assert!((to_display(Units::US, "speed", 1.0) - 3.28084).abs() < 1e-5);
        assert!((to_display(Units::US, "density", 998.2) - 62.316).abs() < 1e-2);
        // k of water 0.598 W/m·K = 0.3455 BTU/h·ft·°F; cp 4182 J/kg·K = 0.9988 BTU/lb·°F; 1 N = 0.22481 lbf.
        assert!((to_display(Units::US, "cond", 0.598) - 0.3455).abs() < 2e-4);
        assert!((to_display(Units::US, "cp", 4182.0) - 0.9988).abs() < 2e-4);
        assert!((to_display(Units::US, "force", 1.0) - 0.224809).abs() < 1e-6);
        assert!((to_display(Units::US, "flux", 1.0) - 0.31700).abs() < 1e-4 && (to_display(Units::US, "power", 1.0) - 3.41214).abs() < 1e-4);
        for u in [Units::SI, Units::US] {
            for k in KINDS {
                let v = from_display(u, k, to_display(u, k, 123.456));
                assert!((v - 123.456).abs() < 1e-9, "{k}");
            }
        }
    }
    #[test]
    fn numbers_read_well() {
        assert_eq!(num(5.57953523384, 4), "5.58");
        assert_eq!(num(0.010618948146, 4), "0.01062");
        assert_eq!(num(1200.0, 3), "1200");
        assert_eq!(num(11954.0, 2), "12000");
        assert_eq!(num(0.0, 3), "0");
        assert_eq!(num(1.2e-7, 2), "1.2e-7");
        assert_eq!(num(f64::NAN, 3), "—");
        assert_eq!(show(Units::SI, "temp", 293.15, 4), "20 °C");
        assert_eq!(show(Units::SI, "ratio", 20.0, 3), "20");
    }
}
