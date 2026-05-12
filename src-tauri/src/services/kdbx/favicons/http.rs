use crate::dto::error::AppError;
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy;
use url::Url;

use super::image::{
    has_known_image_signature, is_potential_image_content_type, looks_like_svg,
    normalize_content_type,
};

pub(super) const FAVICON_MAX_BYTES: usize = 512 * 1024;

pub(super) fn build_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .redirect(Policy::custom(|attempt| {
            if attempt.url().scheme() != "https" {
                attempt.stop()
            } else if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        }))
        .user_agent("MithrilVault/0.1")
        .build()
        .map_err(|error| AppError::Io(error.to_string()))
}

pub(super) async fn fetch_favicon_bytes(
    client: &reqwest::Client,
    fetch_url: &str,
) -> Option<(Vec<u8>, Option<String>)> {
    let mut response = client.get(fetch_url).send().await.ok()?;
    let requested_https = Url::parse(fetch_url).is_ok_and(|url| url.scheme() == "https");
    if requested_https && response.url().scheme() != "https" {
        return None;
    }

    if !response.status().is_success() {
        return None;
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(normalize_content_type);

    if let Some(ref value) = content_type {
        if !is_potential_image_content_type(value) {
            return None;
        }
    }

    if let Some(len) = response.content_length() {
        if len > FAVICON_MAX_BYTES as u64 {
            return None;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len().saturating_add(chunk.len()) > FAVICON_MAX_BYTES {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }

    if bytes.is_empty() {
        return None;
    }

    if !has_known_image_signature(&bytes) && !looks_like_svg(&bytes) {
        return None;
    }

    Some((bytes, content_type))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgba};
    use std::io::{Cursor, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn tiny_png() -> Vec<u8> {
        let image = ImageBuffer::from_pixel(2, 2, Rgba([0_u8, 128, 255, 255]));
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, ImageFormat::Png)
            .expect("write png");
        output.into_inner()
    }

    fn serve_once(response: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("read local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            // Client may close the connection mid-write (e.g. when the
            // streaming cap rejects an oversized body), so a broken pipe
            // here is expected.
            let _ = stream.write_all(&response);
        });

        (format!("http://{address}/favicon.ico"), handle)
    }

    #[test]
    fn fetch_favicon_bytes_accepts_valid_image_responses() {
        let png = tiny_png();
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png; charset=utf-8\r\nContent-Length: "
                .as_slice(),
            png.len().to_string().as_bytes(),
            b"\r\n\r\n",
            png.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched =
            tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url)).expect("fetch png");
        handle.join().expect("server finishes");

        assert_eq!(fetched.0, png);
        assert_eq!(fetched.1.as_deref(), Some("image/png"));
    }

    #[test]
    fn fetch_favicon_bytes_rejects_non_image_content_type() {
        let body = b"<html>not an icon</html>";
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: ".as_slice(),
            body.len().to_string().as_bytes(),
            b"\r\n\r\n",
            body.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_caps_streamed_body_without_content_length() {
        // No Content-Length header — the streaming reader must enforce the
        // cap on its own and short-circuit before buffering the whole body.
        let oversized = vec![0_u8; FAVICON_MAX_BYTES + 1];
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nTransfer-Encoding: chunked\r\n\r\n"
                .as_slice(),
            format!("{:x}\r\n", oversized.len()).as_bytes(),
            oversized.as_slice(),
            b"\r\n0\r\n\r\n",
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        let _ = handle.join();

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_rejects_non_https_redirects() {
        // Server replies with a redirect to an http:// target. The redirect
        // policy should stop following, so the final response stays 302 and
        // is_success() filters it out.
        let response = b"HTTP/1.1 302 Found\r\nLocation: http://example.invalid/favicon.ico\r\nContent-Length: 0\r\n\r\n".to_vec();
        let (url, handle) = serve_once(response);
        let client = build_client().expect("build client");

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_rejects_oversized_icons() {
        let oversized = vec![0_u8; FAVICON_MAX_BYTES + 1];
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: ".as_slice(),
            oversized.len().to_string().as_bytes(),
            b"\r\n\r\n",
            oversized.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }
}
