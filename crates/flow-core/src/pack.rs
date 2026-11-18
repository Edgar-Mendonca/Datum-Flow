//! Compact, exact storage of long number arrays in the `.dflow` file: little-endian bytes in
//! base64 (a third of the size of decimal text, and every bit kept). Also the fingerprint that
//! ties stored results to the model they were solved from.

use crate::model::Model;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            out.push(if k <= c.len() { ALPHABET[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    let t = text.trim_end_matches('=').as_bytes();
    let val = |c: u8| ALPHABET.iter().position(|a| *a == c).map(|p| p as u32);
    if t.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(t.len() * 3 / 4);
    for c in t.chunks(4) {
        let mut n = 0u32;
        for (k, b) in c.iter().enumerate() {
            n |= val(*b)? << (18 - 6 * k);
        }
        for k in 0..c.len() - 1 {
            out.push((n >> (16 - 8 * k)) as u8);
        }
    }
    Some(out)
}

pub fn f64s(v: &[f64]) -> String {
    encode(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}
pub fn u32s(v: &[u32]) -> String {
    encode(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}
pub fn to_f64s(text: &str) -> Option<Vec<f64>> {
    let b = decode(text)?;
    if b.len() % 8 != 0 {
        return None;
    }
    Some(b.chunks(8).map(|c| f64::from_le_bytes(c.try_into().expect("8 bytes"))).collect())
}
pub fn to_u32s(text: &str) -> Option<Vec<u32>> {
    let b = decode(text)?;
    if b.len() % 4 != 0 {
        return None;
    }
    Some(b.chunks(4).map(|c| u32::from_le_bytes(c.try_into().expect("4 bytes"))).collect())
}

/// A fingerprint of everything in the model that the mesh and the results depend on (the
/// drawing, fluid, physics, boundaries, mesh and solver settings), not of how they are shown.
/// Stored results are used only when the model still has the same fingerprint.
pub fn fingerprint(m: &Model) -> String {
    let parts = serde_json::json!([m.shapes, m.fluid, m.physics, m.boundaries, m.mesh, m.solver]);
    let text = serde_json::to_string(&parts).unwrap_or_default();
    // FNV-1a, 64 bits.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrays_round_trip_exactly_and_bad_text_is_refused() {
        for v in [vec![], vec![0.0], vec![1.5, -2.25e-300, f64::MAX, 0.1 + 0.2], (0..1001).map(|i| (i as f64).sin()).collect()] {
            assert_eq!(to_f64s(&f64s(&v)).unwrap(), v);
        }
        assert_eq!(to_u32s(&u32s(&[0, 7, u32::MAX])).unwrap(), vec![0, 7, u32::MAX]);
        assert_eq!(encode(b"Datum"), "RGF0dW0=");
        assert_eq!(decode("RGF0dW0=").unwrap(), b"Datum");
        assert!(decode("RG$0").is_none() && decode("R").is_none() && to_f64s("RGF0dW0=").is_none());
    }
    #[test]
    fn the_fingerprint_follows_the_physics_not_the_view() {
        let m = crate::examples::cylinder();
        let mut n = m.clone();
        n.view.field = "pressure".into();
        n.name = "Other".into();
        n.probes.clear();
        assert_eq!(fingerprint(&m), fingerprint(&n));
        n.fluid.mu *= 2.0;
        assert_ne!(fingerprint(&m), fingerprint(&n));
    }
}
