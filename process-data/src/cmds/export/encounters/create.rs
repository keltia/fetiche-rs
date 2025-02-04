/// Small internal module to manipulate XML data types
///
use super::data::{DataPoint, Encounter};
use kml::{
    types::{AltitudeMode, Coord, Geometry, LineString, LineStyle, Placemark, Style},
    Kml,
};
use std::collections::HashMap;


/// Converts a vector of `DataPoint` objects into a `LineString`.
///
/// This function iterates over the provided points, generating coordinates
/// (`Coord`) for each along with their longitude, latitude, and optional
/// altitude values. It then creates a `LineString` with specific properties
/// such as tessellation, extrusion, and altitude mode set.
///
/// # Arguments
///
/// * `points` - A reference to a vector of `DataPoint`s containing position data.
///
/// # Returns
///
/// An `eyre::Result` containing the generated `LineString` if successful, or an error.
///
/// # Errors
///
/// Will return an error if any issues occur during the processing of points.
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

/// Creates a `Style` KML object with the specified name, color, and line width.
///
/// This function is useful when defining custom styles for placemarks or geometries
/// in the generated KML document. The style specifies visual properties like line
/// color and width.
///
/// # Arguments
///
/// * `name` - A string slice that represents the name or ID for the style.
/// * `colour` - A string slice specifying the color (in KML hex AABBGGRR format).
/// * `size` - A floating-point value specifying the width of the line.
///
/// # Returns
///
/// A `Kml` object containing a `Style`.
///
/// # Examples
///
/// ```
/// use kml::Kml;
///
/// let style = make_style("highlight", "ff0000ff", 2.0);
/// assert!(matches!(style, Kml::Style(..)));
/// ```
///
/// # Errors
///
/// This function does not return an error in its current implementation.
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

/// Generates a list of default KML styles.
///
/// This function parses a hardcoded XML string containing a set of predefined
/// KML styles and style maps to be used in the generated KML documents.
///
/// # Returns
///
/// A vector of `Kml<f64>` elements representing the default styles and style maps.
///
/// # Errors
///
/// This function will panic if the hardcoded XML string cannot be successfully
/// parsed into a valid `Kml` object.
///
#[tracing::instrument]
pub(crate) fn default_styles() -> Vec<Kml<f64>> {
    let str = r##"
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
    "##;

    let s = str.parse::<Kml>().unwrap();
    if let Kml::Document { elements, .. } = s {
        return elements;
    }
    vec![s]
}

/// Converts a trajectory, represented as a collection of `DataPoint` values, into a KML `Placemark`.
///
/// This function creates a `Placemark` that represents the specified trajectory using the given
/// style. The trajectory is visualized as a `LineString` in the KML output.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name or identifier for the placemark (e.g., drone ID, plane ID).
/// * `points` - A reference to a vector of `DataPoint` representing the trajectory.
/// * `style` - A string slice specifying the style (e.g., a KML style URL) to be applied to the geometry.
///
/// # Returns
///
/// A `Result` containing the generated `Kml::Placemark` on success, or an error of type `eyre::Error` if the conversion fails.
///
/// # Errors
///
/// This function will return an error in the following scenarios:
/// - If the conversion of `points` to a `LineString` using the `from_points_to_ls` function fails.
///
/// # Examples
///
/// ```rust
/// let points = vec![
///     DataPoint { latitude: 12.34, longitude: 56.78, altitude: 100.0, timestamp: 123456 },
///     DataPoint { latitude: 13.34, longitude: 57.78, altitude: 150.0, timestamp: 123457 },
/// ];
/// let result = from_traj_to_placemark("My Drone", &points, "#style-id");
/// assert!(result.is_ok());
/// ```
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
        style_url: Some(style.into()),
        attrs: HashMap::from([("styleUrl".into(), style.into())]),
        ..Default::default()
    }))
}

/// Converts a specific `Encounter` point into a KML `Placemark`.
///
/// This function creates a KML `Placemark` object to represent the closest point of the encounter 
/// between two entities, taking their respective coordinates and altitudes into consideration. 
/// The created placemark uses a `LineString` geometry to visually connect the two points, 
/// and applies the specified style to it.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name or identifier for the encounter point.
/// * `res` - A reference to an `Encounter` object describing the encounter details.
/// * `style_url` - A string slice specifying the style URL to be applied to the placemark.
///
/// # Returns
///
/// A `Result` containing the generated `Kml::Placemark` on success, or an error of type `eyre::Error` if the conversion fails.
///
/// # Errors
///
/// This function will return an error in the following scenarios:
/// - If there are issues constructing the `LineString` geometry or its attributes.
/// - If an internal KML-related operation fails during the placemark creation.
///
/// # Examples
///
/// ```rust
/// let encounter = Encounter {
///     drone_lat: 12.34,
///     drone_lon: 56.78,
///     drone_alt_m: 100.0,
///     prox_lat: 13.34,
///     prox_lon: 57.78,
///     prox_alt_m: 150.0,
///     timestamp: 123456,
/// };
/// let result = from_point_to_placemark("Encounter Point", &encounter, "#style-url");
/// assert!(result.is_ok());
/// ```
///
#[tracing::instrument]
pub(crate) fn from_point_to_placemark(
    name: &str,
    res: &Encounter,
    style_url: &str,
) -> eyre::Result<Kml> {
    let points = [
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
        style_url: Some(style_url.into()),
        geometry: Some(Geometry::LineString(ls)),
        attrs: HashMap::new(),
        ..Default::default()
    }))
}
