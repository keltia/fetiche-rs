/// Small internal module to manipulate XML data types
///
use super::data::{DataPoint, Encounter};
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
   	<StyleMap id="default">
		<Pair>
			<key>normal</key>
			<styleUrl>#default0</styleUrl>
		</Pair>
		<Pair>
			<key>highlight</key>
			<styleUrl>#hl</styleUrl>
		</Pair>
	</StyleMap>
	<Style id="default0">
		<LineStyle>
			<color>ff0000ff</color>
		</LineStyle>
		<PolyStyle>
			<color>b3ffffff</color>
		</PolyStyle>
	</Style>
	<Style id="hl">
		<IconStyle>
			<scale>1.2</scale>
		</IconStyle>
		<LineStyle>
			<color>ff0000ff</color>
		</LineStyle>
		<PolyStyle>
			<color>b3ffffff</color>
		</PolyStyle>
	</Style>
	<StyleMap id="msn_ylw-pushpin">
		<Pair>
			<key>normal</key>
			<styleUrl>#sn_ylw-pushpin0</styleUrl>
		</Pair>
		<Pair>
			<key>highlight</key>
			<styleUrl>#sh_ylw-pushpin</styleUrl>
		</Pair>
	</StyleMap>
	<StyleMap id="msn_ylw-pushpin0">
		<Pair>
			<key>normal</key>
			<styleUrl>#sn_ylw-pushpin</styleUrl>
		</Pair>
		<Pair>
			<key>highlight</key>
			<styleUrl>#sh_ylw-pushpin0</styleUrl>
		</Pair>
	</StyleMap>
	<Style id="sh_ylw-pushpin">
		<IconStyle>
			<scale>1.2</scale>
		</IconStyle>
		<BalloonStyle>
		</BalloonStyle>
		<LineStyle>
			<color>ff00ff00</color>
			<width>1.2</width>
		</LineStyle>
	</Style>
	<Style id="sh_ylw-pushpin0">
		<IconStyle>
			<scale>1.2</scale>
		</IconStyle>
		<LineStyle>
			<color>ff00feff</color>
		</LineStyle>
		<PolyStyle>
			<color>b3ffffff</color>
		</PolyStyle>
	</Style>
	<Style id="sn_ylw-pushpin">
		<LineStyle>
			<color>ff00feff</color>
		</LineStyle>
		<PolyStyle>
			<color>b3ffffff</color>
		</PolyStyle>
	</Style>
	<Style id="sn_ylw-pushpin0">
		<LineStyle>
			<color>ff00ff00</color>
			<width>1.2</width>
		</LineStyle>
	</Style>
    "##
        .parse()
        .unwrap()
}

/// Create a `Placemark` given a name (like drone or plane ID) and its trajectory using the
/// requested style.
///
#[tracing::instrument(skip(points, style))]
pub(crate) fn from_traj_to_placemark(
    name: &str,
    points: &Vec<DataPoint>,
    style: &str,
) -> eyre::Result<Kml> {
    let ls = from_points_to_ls(points)?;
    Ok(Kml::Placemark(Placemark {
        name: Some(name.into()),
        geometry: Some(Geometry::LineString(ls)),
        attrs: HashMap::from([("styleUrl".into(), style.into())]),
        ..Default::default()
    }))
}

/// Create a `Placemark` for a specific point, like the closest point between the two, aka encounter.
///
#[tracing::instrument]
pub(crate) fn from_point_to_placemark(
    name: &str,
    res: &Encounter,
    style_url: &str,
) -> eyre::Result<Kml> {
    let points = vec![
        DataPoint {
            latitude: res.drone_lat as f64,
            longitude: res.drone_lon as f64,
            altitude: res.drone_alt_m as f64,
            timestamp: res.timestamp,
        },
        DataPoint {
            latitude: res.prox_lat as f64,
            longitude: res.prox_lon as f64,
            altitude: res.prox_alt_m as f64,
            timestamp: res.timestamp,
        },
    ];
    let coords = points
        .iter()
        .map(|p| Coord::new(p.longitude, p.latitude, Some(p.altitude)))
        .collect::<Vec<_>>();

    let ls = LineString {
        tessellate: false,
        extrude: false,
        altitude_mode: AltitudeMode::Absolute,
        coords,
        ..Default::default()
    };

    Ok(Kml::Placemark(Placemark {
        name: Some(name.into()),
        description: Some("Closest point".into()),
        geometry: Some(Geometry::LineString(ls)),
        attrs: HashMap::from([("styleUrl".into(), style_url.into())]),
        ..Default::default()
    }))
}
