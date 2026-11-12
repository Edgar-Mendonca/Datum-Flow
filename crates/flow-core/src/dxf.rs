//! Reading outlines from a DXF drawing: LINE, ARC, CIRCLE and LWPOLYLINE entities only. Lines and
//! arcs that meet end to end are joined into closed outlines; the largest becomes the flow
//! region and outlines inside it become bodies cut out of it. Anything else is left out and
//! counted in the notes.

use crate::model::Geom;

/// One outline read from a drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub name: String,
    pub hole: bool,
    pub geom: Geom,
}

/// A piece of an outline: from a to b, with the bulge of its arc (0 = straight).
#[derive(Clone, Copy)]
struct Piece {
    a: [f64; 2],
    b: [f64; 2],
    bulge: f64,
}

/// The largest drawing read (bytes) and the most entities used.
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_PIECES: usize = 20_000;

/// Read the outlines of a DXF text. `scale` turns the drawing's units into metres (0.001 for a
/// drawing in millimetres). Returns the outlines and notes about what was left out.
pub fn read(text: &str, scale: f64) -> Result<(Vec<Outline>, Vec<String>), String> {
    if text.len() > MAX_BYTES {
        return Err("This drawing is too large to import.".into());
    }
    if !(scale.is_finite() && scale > 0.0) {
        return Err("The drawing's unit must be a length above zero.".into());
    }
    // A DXF file is pairs of lines: a group code, then its value.
    let mut pairs: Vec<(i32, &str)> = vec![];
    let mut lines = text.lines();
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        match code.trim().parse::<i32>() {
            Ok(c) => pairs.push((c, value.trim())),
            Err(_) => return Err("This is not a DXF drawing in text form (binary DXF and DWG files cannot be read).".into()),
        }
    }
    let start = pairs.iter().position(|p| p.0 == 2 && p.1 == "ENTITIES").ok_or("This drawing has no entities section; it may be empty or not a DXF file.")?;
    let (mut pieces, mut closed, mut skipped): (Vec<Piece>, Vec<Geom>, std::collections::BTreeMap<String, usize>) = (vec![], vec![], Default::default());
    let mut i = start + 1;
    let num = |v: &str| v.parse::<f64>().ok().filter(|x| x.is_finite() && x.abs() < 1e12);
    while i < pairs.len() {
        if pairs[i].0 != 0 {
            i += 1;
            continue;
        }
        let kind = pairs[i].1;
        if kind == "ENDSEC" {
            break;
        }
        let mut j = i + 1;
        while j < pairs.len() && pairs[j].0 != 0 {
            j += 1;
        }
        let body = &pairs[i + 1..j];
        let get = |code: i32| body.iter().find(|p| p.0 == code).and_then(|p| num(p.1));
        let p = |x: f64, y: f64| [x * scale, y * scale];
        match kind {
            "LINE" => if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (get(10), get(20), get(11), get(21)) {
                pieces.push(Piece { a: p(x1, y1), b: p(x2, y2), bulge: 0.0 });
            },
            "ARC" => if let (Some(cx), Some(cy), Some(r), Some(a0), Some(a1)) = (get(10), get(20), get(40), get(50), get(51)) {
                // Counter-clockwise from the start angle to the end angle (degrees).
                let sweep = (a1 - a0).rem_euclid(360.0);
                let sweep = if sweep == 0.0 { 360.0 } else { sweep }.to_radians();
                let at = |deg: f64| p(cx + r * deg.to_radians().cos(), cy + r * deg.to_radians().sin());
                if r > 0.0 && sweep < 6.2 {
                    pieces.push(Piece { a: at(a0), b: at(a1), bulge: (sweep / 4.0).tan() });
                }
            },
            "CIRCLE" => if let (Some(cx), Some(cy), Some(r)) = (get(10), get(20), get(40)) {
                if r > 0.0 { closed.push(Geom::Circle { c: p(cx, cy), r: r * scale }); }
            },
            "LWPOLYLINE" => {
                let is_closed = body.iter().find(|p| p.0 == 70).and_then(|p| p.1.parse::<i32>().ok()).unwrap_or(0) & 1 == 1;
                let (mut pts, mut bulge): (Vec<[f64; 2]>, Vec<f64>) = (vec![], vec![]);
                let mut x = None;
                for (code, value) in body {
                    match code {
                        10 => x = num(value),
                        20 => if let (Some(px), Some(py)) = (x.take(), num(value)) { pts.push(p(px, py)); bulge.push(0.0); },
                        42 => if let (Some(b), Some(v)) = (bulge.last_mut(), num(value)) { *b = v; },
                        _ => {}
                    }
                }
                if is_closed && pts.len() >= 2 {
                    closed.push(Geom::Poly { pts, bulge });
                } else {
                    for k in 0..pts.len().saturating_sub(1) {
                        pieces.push(Piece { a: pts[k], b: pts[k + 1], bulge: bulge[k] });
                    }
                }
            }
            other => *skipped.entry(other.to_string()).or_default() += 1,
        }
        if pieces.len() > MAX_PIECES || closed.len() > 500 {
            return Err("This drawing has too many lines to import as outlines. Export only the outline's layer.".into());
        }
        i = j;
    }
    let mut notes: Vec<String> = skipped.iter().map(|(k, n)| format!("{n} {k} {} left out (only lines, arcs, circles and polylines are read).", if *n == 1 { "entity was" } else { "entities were" })).collect();
    // Join pieces end to end into closed outlines.
    let size = pieces.iter().flat_map(|p| [p.a, p.b]).fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])]);
    let tol = 1e-6 * ((size[2] - size[0]).hypot(size[3] - size[1])).max(1e-12);
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) <= tol;
    pieces.retain(|p| !near(p.a, p.b) || p.bulge != 0.0);
    let mut used = vec![false; pieces.len()];
    let mut open = 0;
    for s in 0..pieces.len() {
        if used[s] {
            continue;
        }
        used[s] = true;
        let mut chain = vec![pieces[s]];
        loop {
            let end = chain[chain.len() - 1].b;
            if near(end, chain[0].a) {
                break;
            }
            let next = (0..pieces.len()).find(|&k| !used[k] && (near(pieces[k].a, end) || near(pieces[k].b, end)));
            match next {
                Some(k) => {
                    used[k] = true;
                    let q = pieces[k];
                    // A piece met at its far end is walked backwards: its arc bulges the other way.
                    chain.push(if near(q.a, end) { q } else { Piece { a: q.b, b: q.a, bulge: -q.bulge } });
                }
                None => break,
            }
        }
        if near(chain[chain.len() - 1].b, chain[0].a) && chain.len() >= 2 {
            closed.push(Geom::Poly { pts: chain.iter().map(|c| c.a).collect(), bulge: chain.iter().map(|c| c.bulge).collect() });
        } else {
            open += 1;
        }
    }
    if open > 0 {
        notes.push(format!("{open} open {} left out: an outline must close on itself.", if open == 1 { "run of lines was" } else { "runs of lines were" }));
    }
    let before = closed.len();
    closed.retain(|g| g.problem().is_none());
    if closed.len() != before {
        notes.push(format!("{} outline(s) were left out because they cross themselves or enclose no area.", before - closed.len()));
    }
    if closed.is_empty() {
        return Err(format!("No closed outline was found in this drawing.{}", if notes.is_empty() { String::new() } else { format!(" {}", notes.join(" ")) }));
    }
    // The largest outline is the flow region; outlines whose first point lies inside another are bodies.
    closed.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
    let rings: Vec<Vec<[f64; 2]>> = closed.iter().map(|g| g.outline(f64::INFINITY, 32).0).collect();
    let inside = |poly: &[[f64; 2]], p: [f64; 2]| {
        let (mut c, n) = (false, poly.len());
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (poly[i], poly[j]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] { c = !c; }
            j = i;
        }
        c
    };
    let (mut regions, mut bodies) = (0, 0);
    let out = closed.iter().enumerate().map(|(i, g)| {
        // Inside an odd number of larger outlines: a body cut out.
        let depth = (0..i).filter(|&k| inside(&rings[k], rings[i][0])).count();
        let hole = depth % 2 == 1;
        let name = if hole { bodies += 1; format!("Body {bodies}") } else { regions += 1; if regions == 1 { "Region".to_string() } else { format!("Region {regions}") } };
        Outline { name, hole, geom: g.clone() }
    }).collect();
    Ok((out, notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::arc;

    fn radius(a: [f64; 2], b: [f64; 2], bulge: f64) -> f64 {
        arc(a, b, bulge).1
    }
    fn dxf(entities: &str) -> String {
        format!("0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n")
    }
    fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
        format!("0\nLINE\n8\n0\n10\n{x1}\n20\n{y1}\n11\n{x2}\n21\n{y2}\n")
    }

    #[test]
    fn lines_arcs_and_circles_become_a_region_with_bodies() {
        // A 200 × 100 mm slot-ended plate: two lines and two half-circle arcs, drawn in any order
        // and direction, with a circle and a closed polyline inside, and a text to skip.
        let mut e = String::new();
        e += &line(200.0, 100.0, 0.0, 100.0);
        e += &line(0.0, 0.0, 200.0, 0.0);
        e += "0\nARC\n10\n200\n20\n50\n40\n50\n50\n270\n51\n90\n";
        e += "0\nARC\n10\n0\n20\n50\n40\n50\n50\n90\n51\n270\n";
        e += "0\nCIRCLE\n10\n50\n20\n50\n40\n10\n";
        e += "0\nLWPOLYLINE\n90\n4\n70\n1\n10\n120\n20\n40\n10\n160\n20\n40\n10\n160\n20\n60\n10\n120\n20\n60\n";
        e += "0\nTEXT\n1\nhello\n";
        e += &line(500.0, 500.0, 600.0, 500.0);
        let (out, notes) = read(&dxf(&e), 0.001).unwrap();
        assert_eq!(out.len(), 3);
        assert!(out[0].name == "Region" && !out[0].hole && out[1].hole && out[2].hole, "{out:?}");
        // Area: the rectangle plus a full circle of radius 50 mm, in m².
        let want = 0.2 * 0.1 + std::f64::consts::PI * 0.05 * 0.05;
        assert!((out[0].geom.area().abs() - want).abs() < 1e-9, "{} {want}", out[0].geom.area());
        let Geom::Poly { pts, bulge } = &out[0].geom else { panic!() };
        let k = bulge.iter().position(|b| *b != 0.0).unwrap();
        assert!((radius(pts[k], pts[(k + 1) % pts.len()], bulge[k]) - 0.05).abs() < 1e-12 && bulge.iter().filter(|b| **b != 0.0).count() == 2);
        assert!(matches!(out[2].geom, Geom::Circle { r, .. } if (r - 0.01).abs() < 1e-15) || matches!(out[1].geom, Geom::Circle { .. }));
        assert!(notes.iter().any(|n| n.contains("1 TEXT entity was left out")) && notes.iter().any(|n| n.contains("1 open run of lines was left out")), "{notes:?}");
    }

    #[test]
    fn drawings_that_cannot_be_used_say_why() {
        assert!(read("hello\nworld\n", 1.0).unwrap_err().contains("not a DXF drawing in text form"));
        assert!(read("0\nSECTION\n2\nHEADER\n0\nENDSEC\n", 1.0).unwrap_err().contains("no entities section"));
        assert!(read(&dxf(&line(0.0, 0.0, 1.0, 0.0)), 1.0).unwrap_err().contains("No closed outline"));
        assert!(read(&dxf(""), 0.0).unwrap_err().contains("above zero"));
        // Garbled numbers and a cut-off file do not panic.
        let bad = dxf("0\nLINE\n10\nabc\n20\n1e999\n11\n\n21\n3\n0\nCIRCLE\n10\n0\n20\n0\n40\n-5\n0\nLWPOLYLINE\n70\n1\n10\n1\n");
        assert!(read(&bad, 1.0).is_err());
        let good = dxf(&(line(0.0, 0.0, 1.0, 0.0) + &line(1.0, 0.0, 0.0, 1.0) + &line(0.0, 1.0, 0.0, 0.0)));
        for cut in (0..good.len()).step_by(7) {
            let _ = read(&good[..cut], 1.0);
        }
        assert_eq!(read(&good, 1.0).unwrap().0.len(), 1);
    }
}
