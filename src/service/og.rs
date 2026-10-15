use crate::{db, Result};
use deadpool_sqlite::Pool;
use futures_util::future::join_all;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::overlay;
use image::ImageEncoder;
use image::Rgba;
use image::RgbaImage;
use include_dir::include_dir;
use include_dir::Dir;
use std::f64::consts::PI;
use std::time::Duration;
use tracing::warn;

static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

pub const OG_WIDTH: i64 = 600;
pub const OG_HEIGHT: i64 = 315;

/// Zoom the static map is rendered at.
const ZOOM: u8 = 17;
/// Edge length of a raster XYZ tile, in pixels.
const TILE_SIZE: u32 = 256;
/// Raster XYZ tile template. Single source of truth so the provider can be
/// swapped in one place.
const TILE_URL_TEMPLATE: &str = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
/// Some tile providers (notably OSM) require an identifying User-Agent.
const USER_AGENT: &str = "btcmap-api (https://btcmap.org)";
/// Colour shown where a tile could not be fetched.
const BACKGROUND: Rgba<u8> = Rgba([221, 221, 221, 255]);
/// Offset from the marker sprite's top-left corner to its tip. The tip is
/// anchored on the element's coordinate, matching the previous staticmap setup.
const MARKER_TIP_OFFSET: (i64, i64) = (20, 53);
/// Per-tile download timeout.
const TILE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct OgImagePayload {
    pub width: i64,
    pub height: i64,
    pub data: Vec<u8>,
}

pub async fn element_og(id: &str, pool: &Pool) -> Result<OgImagePayload> {
    let Ok(element) = db::main::element::queries::select_by_id_or_osm_id(id, pool).await else {
        return Err("Element not found".into());
    };
    render(element.lat(), element.lon()).await
}

async fn render(lat: f64, lon: f64) -> Result<OgImagePayload> {
    let width = OG_WIDTH as u32;
    let height = OG_HEIGHT as u32;

    // The Web-Mercator world is `TILE_SIZE * 2^ZOOM` pixels wide and tall.
    let world = TILE_SIZE as f64 * 2f64.powi(ZOOM as i32);
    // Top-left corner of the output image, in world pixels. The element sits at
    // the centre of the image.
    let origin = (
        lon_to_world_x(lon, world) - width as f64 / 2.0,
        lat_to_world_y(lat, world) - height as f64 / 2.0,
    );

    let (x_min, y_min, x_max, y_max) = tile_range(origin.0, origin.1, width, height);
    let max_index = (1i64 << ZOOM) - 1;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TILE_TIMEOUT)
        .build()?;

    // Fetch every covering tile in parallel. A tile that fails is skipped and
    // the background shows through instead of failing the whole image.
    let mut fetches = Vec::new();
    for ty in y_min..=y_max {
        for tx in x_min..=x_max {
            let client = client.clone();
            fetches.push(async move {
                // Longitude wraps around the world, latitude does not.
                if !(0..=max_index).contains(&ty) {
                    return None;
                }
                let wrapped_x = tx.rem_euclid(max_index + 1);
                let url = TILE_URL_TEMPLATE
                    .replace("{z}", &ZOOM.to_string())
                    .replace("{x}", &wrapped_x.to_string())
                    .replace("{y}", &ty.to_string());
                match fetch_tile(&client, &url).await {
                    Ok(bytes) => Some((tx, ty, bytes)),
                    Err(err) => {
                        warn!(%url, error = %err, "failed to fetch OG map tile");
                        None
                    }
                }
            });
        }
    }
    let tiles: Vec<(i64, i64, Vec<u8>)> = join_all(fetches).await.into_iter().flatten().collect();

    let marker = ICONS_DIR
        .get_file("marker.png")
        .map(|file| file.contents().to_vec());

    actix_web::web::block(move || compose(width, height, origin, tiles, marker)).await?
}

async fn fetch_tile(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = client.get(url).send().await?.error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

fn compose(
    width: u32,
    height: u32,
    origin: (f64, f64),
    tiles: Vec<(i64, i64, Vec<u8>)>,
    marker: Option<Vec<u8>>,
) -> Result<OgImagePayload> {
    let mut canvas = RgbaImage::from_pixel(width, height, BACKGROUND);

    for (tx, ty, bytes) in tiles {
        let tile = match image::load_from_memory(&bytes) {
            Ok(tile) => tile.to_rgba8(),
            Err(err) => {
                warn!(error = %err, "failed to decode OG map tile");
                continue;
            }
        };
        let dst_x = (tx as f64 * TILE_SIZE as f64 - origin.0).round() as i64;
        let dst_y = (ty as f64 * TILE_SIZE as f64 - origin.1).round() as i64;
        overlay(&mut canvas, &tile, dst_x, dst_y);
    }

    if let Some(marker) = marker {
        if let Ok(marker) = image::load_from_memory(&marker) {
            let marker = marker.to_rgba8();
            let x = width as i64 / 2 - MARKER_TIP_OFFSET.0;
            let y = height as i64 / 2 - MARKER_TIP_OFFSET.1;
            overlay(&mut canvas, &marker, x, y);
        }
    }

    let rgb = image::DynamicImage::ImageRgba8(canvas).to_rgb8();
    let mut data = Vec::new();
    JpegEncoder::new_with_quality(&mut data, 80).write_image(
        &rgb,
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(OgImagePayload {
        width: width as i64,
        height: height as i64,
        data,
    })
}

/// Longitude to world X position, in pixels.
fn lon_to_world_x(lon: f64, world: f64) -> f64 {
    (lon + 180.0) / 360.0 * world
}

/// Latitude to world Y position, in pixels (Web-Mercator).
fn lat_to_world_y(lat: f64, world: f64) -> f64 {
    let lat = lat.to_radians();
    (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / PI) / 2.0 * world
}

/// Inclusive tile index range `(x_min, y_min, x_max, y_max)` covering an image
/// of the given size whose top-left corner is at `(origin_x, origin_y)` in
/// world pixels.
fn tile_range(origin_x: f64, origin_y: f64, width: u32, height: u32) -> (i64, i64, i64, i64) {
    let tile = TILE_SIZE as f64;
    (
        (origin_x / tile).floor() as i64,
        (origin_y / tile).floor() as i64,
        ((origin_x + width as f64 - 1.0) / tile).floor() as i64,
        ((origin_y + height as f64 - 1.0) / tile).floor() as i64,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn world_coordinates() {
        let world = 256.0;
        assert!((lon_to_world_x(-180.0, world) - 0.0).abs() < 1e-9);
        assert!((lon_to_world_x(0.0, world) - 128.0).abs() < 1e-9);
        assert!((lon_to_world_x(180.0, world) - 256.0).abs() < 1e-9);
        // The equator is halfway down the world.
        assert!((lat_to_world_y(0.0, world) - 128.0).abs() < 1e-9);
    }

    #[test]
    fn tile_range_covers_image() {
        // A 600x315 image at the origin spans tiles 0..=2 by 0..=1.
        assert_eq!((0, 0, 2, 1), tile_range(0.0, 0.0, 600, 315));
        // Shifted by exactly one tile in X.
        assert_eq!((1, 0, 3, 1), tile_range(256.0, 0.0, 600, 315));
        // A fractional origin still floors to the containing tile.
        assert_eq!((0, 0, 3, 2), tile_range(255.9, 255.9, 600, 315));
    }
}
