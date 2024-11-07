/// Small internal module to manipulate XML data types
///
use super::data::DataPoint;
use kml::{
    types::{AltitudeMode, Coord, Geometry, LineString, LineStyle, Placemark, Style},
    Kml,
};
use std::collections::HashMap;


/// Generate a `LineString` given a list of (x,y,z) points.
///
#[tracing::instrument]
fn from_points_to_ls(points: &Vec<DataPoint>) -> eyre::Result<LineString> {
    let coords = points
        .iter()
        .map(|p| Coord::new(p.longitude, p.latitude, Some(p.altitude)))
        .collect::<Vec<_>>();

    Ok(LineString {
        tessellate: false,
        extrude: true,
        altitude_mode: AltitudeMode::Absolute,
        coords,
        ..Default::default()
    })
}

/// Create a `Style`  entry for a `Placemark`
///
#[tracing::instrument]
pub(crate) fn make_style(name: &str, colour: &str, size: f64) -> Kml {
    Kml::Style(Style {
        id: Some(name.into()),
        line: LineStyle {
            color: colour.into(),
            width: size,
            ..Default::default()
        }
            .into(),
        ..Default::default()
    })
}

/// Generate a default style list.
///
#[tracing::instrument]
pub(crate) fn default_styles() -> Kml {
    r##"
      <Style id="lineStyleNormal">
        <LineStyle><color>64F0FA14</color><width>4</width></LineStyle>
        <PolyStyle><color>32F0F014</color></PolyStyle>
      </Style>
      <Style id="lineStyleHighlight">
        <LineStyle><color>641400E6</color><width>4</width></LineStyle>
        <PolyStyle><color>321400FF</color></PolyStyle>
        </Style>
      <StyleMap id="lineStyle">
        <Pair><key>normal</key><styleUrl>#lineStyleNormal</styleUrl></Pair>
        <Pair><key>highlight</key><styleUrl>#lineStyleHighlight</styleUrl></Pair>
      /StyleMap>
      <StyleMap id="pointStyle">
        <Pair><key>normal</key><styleUrl>#pointStyleNormal</styleUrl></Pair>
        <Pair><key>highlight</key><styleUrl>#pointStyleHighlight</styleUrl></Pair>
      /StyleMap>
      <Style id="pointStyleNormal">
        <IconStyle><color>ff00ffff</color></IconStyle>
        <LabelStyle><color>ff00ffff</color></LabelStyle>
      /Style>
      <Style id="pointStyleHighlight">
        <IconStyle><color>ff0000ff</color><scale>1.2</scale></IconStyle>
        <LabelStyle><color>ff0000ff</color></LabelStyle>
      /Style>
    "##.parse().unwrap()
}

/// Create a `Placemark` given a name (like drone or plane ID) and its trajectory using the
/// requested style.
///
pub(crate) fn from_traj_to_placemark(
    name: &str,
    points: &Vec<DataPoint>,
    style: &str,
) -> eyre::Result<Kml> {
    let ls = from_points_to_ls(points)?;
    let style_url = format!("#{style}");
    Ok(Kml::Placemark(Placemark {
        name: Some(name.into()),
        geometry: Some(Geometry::LineString(ls)),
        attrs: HashMap::from([("styleUrl".into(), style_url)]),
        ..Default::default()
    }))
}
