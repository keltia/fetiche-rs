//! This example demonstrates how to generate a KML file from a series of data points
//! representing a trajectory. It showcases converting the trajectory into KML structures 
//! such as LineString and Placemark, and serializing them to produce a KML document.
//!
//! Additionally, it showcases how to manipulate colors using the `colorsys` crate.
//!
use chrono::{DateTime, Utc};
use colorsys::{ColorAlpha, Rgb};
use dateparser::parse;
use eyre::Result;
use klickhouse::Row;
use kml::types::{AltitudeMode, Coord, Geometry, LineString, Placemark};
use kml::Kml::Document;
use kml::{Kml, KmlDocument, KmlVersion, KmlWriter};
use serde::Serialize;

#[derive(Clone, Debug, Row, Serialize)]
struct DataPoint {
    timestamp: DateTime<Utc>,
    latitude: f32,
    longitude: f32,
    altitude: f32,
}

fn from_traj_to_ls(points: &Vec<DataPoint>) -> Result<LineString> {
    let coords = points
        .into_iter()
        .map(|p| {
            Coord::new(
                p.longitude as f64,
                p.latitude as f64,
                Some(p.altitude as f64),
            )
        })
        .collect::<Vec<_>>();

    let ls = LineString {
        tessellate: false,
        extrude: true,
        altitude_mode: AltitudeMode::Absolute,
        coords: coords.into(),
        ..Default::default()
    };
    Ok(ls)
}

fn from_traj_to_placemark(name: &str, points: &Vec<DataPoint>) -> Result<Kml> {
    let ls = from_traj_to_ls(points)?;
    let pm = Placemark {
        name: Some(name.into()),
        geometry: Some(Geometry::LineString(ls.into())),
        ..Default::default()
    };

    let pm = Kml::Placemark(pm);
    Ok(pm)
}

fn main() -> Result<()> {
    let dp = vec![
        DataPoint {
            timestamp: parse("2023-05-23 05:01:59.190000000").unwrap(),
            latitude: 49.6332976408,
            longitude: 6.2299531326,
            altitude: 0.0,
        },
        DataPoint {
            timestamp: parse("2023-05-23 05:02:00.980000000").unwrap(),
            latitude: 49.6329018474,
            longitude: 6.2288953364,
            altitude: 0.0,
        },
        DataPoint {
            timestamp: parse("2023-05-23 05:02:03.540000000").unwrap(),
            latitude: 49.6322936565,
            longitude: 6.2272241525,
            altitude: 1400.0,
        },
        DataPoint {
            timestamp: parse("2023-05-23 05:02:05.570000000").unwrap(),
            latitude: 49.6317443065,
            longitude: 6.2257786095,
            altitude: 1400.0,
        },
    ];
    let foo = from_traj_to_placemark("FOO", &dp)?;

    let doc = Document {
        attrs: [("name".to_string(), "Flight Path".to_string())].into(),
        elements: vec![foo.into()],
    };

    let doc = KmlDocument {
        attrs: [("name".to_string(), "foo.kml".to_string())].into(),
        elements: vec![doc.into()],
        version: KmlVersion::V23,
    };

    let doc = Kml::KmlDocument(doc.into());

    let mut buf = vec![];
    let mut w = KmlWriter::from_writer(&mut buf);
    w.write(&doc)?;

    println!("{}", String::from_utf8(buf)?);

    let mut red = Rgb::from((255., 0., 0.));
    red.opacify(-0.3);
    let green = Rgb::from((0., 255., 0., 1.0));

    dbg!(&red);
    let red_str = format!("#{}ff", red.to_hex_string());
    let green_str = format!("#{}ff", green.to_hex_string());

    println!("{}", red_str);
    println!("{}", green_str);

    Ok(())
}
