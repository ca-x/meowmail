use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[cfg(meowmail_embedded_web)]
use axum::{
    body::Body,
    http::{HeaderValue, header},
};

#[cfg(meowmail_embedded_web)]
use rust_embed::RustEmbed;

#[cfg(meowmail_embedded_web)]
#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct EmbeddedAssets;

pub async fn serve(request: Request) -> Response {
    let path = request.uri().path().trim_start_matches('/');
    if path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    #[cfg(meowmail_embedded_web)]
    {
        return embedded(path).unwrap_or_else(|| StatusCode::NOT_FOUND.into_response());
    }
    #[cfg(not(meowmail_embedded_web))]
    {
        let _ = path;
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "The debug backend serves APIs only. Run `npm --prefix web run dev` for the Web UI.",
        )
            .into_response()
    }
}

#[cfg(meowmail_embedded_web)]
fn embedded(path: &str) -> Option<Response> {
    let asset_path = if path.is_empty() { "index.html" } else { path };
    let asset = EmbeddedAssets::get(asset_path).or_else(|| EmbeddedAssets::get("index.html"))?;
    let actual_path = if EmbeddedAssets::get(asset_path).is_some() {
        asset_path
    } else {
        "index.html"
    };
    let mime = mime_guess::from_path(actual_path).first_or_octet_stream();
    let mut response = Body::from(asset.data.into_owned()).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).ok()?,
    );
    let cache = match actual_path {
        "index.html" => "no-cache",
        path if path.starts_with("assets/") => "public, max-age=31536000, immutable",
        _ => "public, max-age=3600",
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    Some(response)
}
