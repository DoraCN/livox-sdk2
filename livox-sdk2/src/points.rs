use crate::ffi;

/// A single parsed point in Cartesian coordinates.
///
/// Coordinates are in **meters**. `reflectivity` and `tag` carry the raw
/// sensor values (0–255).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub reflectivity: u8,
    pub tag: u8,
}

impl Point {
    /// True for the special Livox "0x100/0x101 invalid points" convention
    /// used to mark the blind spot / invalid returns.
    pub fn is_valid(&self) -> bool {
        self.tag == 0 && self.reflectivity != 0xFF
    }
}

/// Pixel/byte sizes of the raw point records per data type.
const CARTESIAN_HIGH_SIZE: usize = 14;
const CARTESIAN_LOW_SIZE: usize = 8;
const SPHERICAL_SIZE: usize = 12;
const DOUBLE_ECHO_SIZE: usize = 28;

impl crate::Packet<'_> {
    /// Parses the raw payload into a `Vec<Point>`.
    ///
    /// The point format is selected automatically from the packet's
    /// `data_type` field (Cartesian high/low, spherical, or double-echo).
    /// IMU packets yield an empty vector.
    pub fn points(&self) -> Vec<Point> {
        let data = self.data();
        match self.data_type() {
            t if t == ffi::LivoxLidarPointDataType_kLivoxLidarCartesianCoordinateHighData as u8 => {
                parse_cartesian_high(data)
            }
            t if t == ffi::LivoxLidarPointDataType_kLivoxLidarCartesianCoordinateLowData as u8 => {
                parse_cartesian_low(data)
            }
            t if t == ffi::LivoxLidarPointDataType_kLivoxLidarSphericalCoordinateData as u8 => {
                parse_spherical(data)
            }
            t if t == ffi::LivoxLidarPointDataType_kLivoxLidarDoubleEchoData as u8 => {
                parse_double_echo(data)
            }
            _ => Vec::new(),
        }
    }
}

fn parse_cartesian_high(data: &[u8]) -> Vec<Point> {
    let mut out = Vec::with_capacity(data.len() / CARTESIAN_HIGH_SIZE);
    for c in data.chunks_exact(CARTESIAN_HIGH_SIZE) {
        out.push(Point {
            x: i32::from_le_bytes(c[0..4].try_into().unwrap()) as f64 * 0.001,
            y: i32::from_le_bytes(c[4..8].try_into().unwrap()) as f64 * 0.001,
            z: i32::from_le_bytes(c[8..12].try_into().unwrap()) as f64 * 0.001,
            reflectivity: c[12],
            tag: c[13],
        });
    }
    out
}

fn parse_cartesian_low(data: &[u8]) -> Vec<Point> {
    let mut out = Vec::with_capacity(data.len() / CARTESIAN_LOW_SIZE);
    for c in data.chunks_exact(CARTESIAN_LOW_SIZE) {
        out.push(Point {
            x: i16::from_le_bytes(c[0..2].try_into().unwrap()) as f64 * 0.01,
            y: i16::from_le_bytes(c[2..4].try_into().unwrap()) as f64 * 0.01,
            z: i16::from_le_bytes(c[4..6].try_into().unwrap()) as f64 * 0.01,
            reflectivity: c[6],
            tag: c[7],
        });
    }
    out
}

/// Spherical → Cartesian conversion.
///
/// Raw layout: `depth` (mm, `u32`), `theta` (azimuth, 0.01°), `phi`
/// (elevation, 0.01°), `reflectivity`, `tag`.
fn parse_spherical(data: &[u8]) -> Vec<Point> {
    const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
    let mut out = Vec::with_capacity(data.len() / SPHERICAL_SIZE);
    for c in data.chunks_exact(SPHERICAL_SIZE) {
        let depth = u32::from_le_bytes(c[0..4].try_into().unwrap()) as f64 * 0.001;
        let theta = u16::from_le_bytes(c[4..6].try_into().unwrap()) as f64 * 0.01 * DEG2RAD;
        let phi = u16::from_le_bytes(c[6..8].try_into().unwrap()) as f64 * 0.01 * DEG2RAD;
        let cp = phi.cos();
        out.push(Point {
            x: depth * cp * theta.cos(),
            y: depth * cp * theta.sin(),
            z: depth * phi.sin(),
            reflectivity: c[8],
            tag: c[9],
        });
    }
    out
}

fn parse_double_echo(data: &[u8]) -> Vec<Point> {
    let mut out = Vec::with_capacity(data.len() / DOUBLE_ECHO_SIZE);
    for c in data.chunks_exact(DOUBLE_ECHO_SIZE) {
        out.push(point_from_high(&c[0..14]));
        out.push(point_from_high(&c[14..28]));
    }
    out
}

/// Reads one Cartesian-high record (14 bytes) from `c`.
fn point_from_high(c: &[u8]) -> Point {
    Point {
        x: i32::from_le_bytes(c[0..4].try_into().unwrap()) as f64 * 0.001,
        y: i32::from_le_bytes(c[4..8].try_into().unwrap()) as f64 * 0.001,
        z: i32::from_le_bytes(c[8..12].try_into().unwrap()) as f64 * 0.001,
        reflectivity: c[12],
        tag: c[13],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cartesian_high() {
        // x=1000mm, y=-500mm, z=250mm, refl=128, tag=0
        let mut buf = [0u8; 14];
        buf[0..4].copy_from_slice(&1000i32.to_le_bytes());
        buf[4..8].copy_from_slice(&(-500i32).to_le_bytes());
        buf[8..12].copy_from_slice(&250i32.to_le_bytes());
        buf[12] = 128;
        let pts = parse_cartesian_high(&buf);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].x, 1.0);
        assert_eq!(pts[0].y, -0.5);
        assert_eq!(pts[0].z, 0.25);
        assert_eq!(pts[0].reflectivity, 128);
    }

    #[test]
    fn cartesian_low() {
        // x=100cm, y=-50cm, z=25cm, refl=64, tag=0
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&100i16.to_le_bytes());
        buf[2..4].copy_from_slice(&(-50i16).to_le_bytes());
        buf[4..6].copy_from_slice(&25i16.to_le_bytes());
        buf[6] = 64;
        let pts = parse_cartesian_low(&buf);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].x, 1.0);
        assert_eq!(pts[0].y, -0.5);
        assert_eq!(pts[0].z, 0.25);
    }

    #[test]
    fn spherical() {
        // depth=2000mm, theta=0 (0°), phi=3000 (30°), refl=200, tag=0
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&2000u32.to_le_bytes());
        buf[4..6].copy_from_slice(&0u16.to_le_bytes());
        buf[6..8].copy_from_slice(&3000u16.to_le_bytes());
        buf[8] = 200;
        let pts = parse_spherical(&buf);
        assert_eq!(pts.len(), 1);
        let p = pts[0];
        // x = 2*cos(30°)*cos(0°) ≈ 1.732
        assert!((p.x - 1.7320508).abs() < 1e-6);
        assert!((p.y).abs() < 1e-12);
        // z = 2*sin(30°) = 1.0
        assert!((p.z - 1.0).abs() < 1e-12);
        assert_eq!(p.reflectivity, 200);
    }

    #[test]
    fn double_echo() {
        let mut buf = [0u8; 28];
        buf[0..4].copy_from_slice(&1000i32.to_le_bytes()); // p1.x = 1m
        buf[14..18].copy_from_slice(&(-2000i32).to_le_bytes()); // p2.x = -2m
        buf[27] = 7; // p2.tag
        let pts = parse_double_echo(&buf);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].x, 1.0);
        assert_eq!(pts[1].x, -2.0);
        assert_eq!(pts[1].tag, 7);
    }

    #[test]
    fn truncated_data_yields_partial_or_empty() {
        assert!(parse_cartesian_high(&[0u8; 13]).is_empty());
        assert_eq!(parse_cartesian_high(&[0u8; 28]).len(), 2);
    }

    #[test]
    fn invalid_point_heuristic() {
        // reflectivity 0xFF marks invalid/Livox blind-spot convention.
        let p = Point { x: 1.0, y: 0.0, z: 0.0, reflectivity: 0xFF, tag: 0 };
        assert!(!p.is_valid());
        let ok = Point { x: 1.0, y: 0.0, z: 0.0, reflectivity: 50, tag: 0 };
        assert!(ok.is_valid());
    }
}
