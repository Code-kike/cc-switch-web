//! HTTP content-encoding helpers.
//!
//! Reqwest auto-decompression is disabled so the proxy can preserve the client's
//! original accept-encoding header. Request and response paths therefore share
//! this explicit decoding logic.

use axum::http::header::HeaderMap;
use std::io::Read;

fn split_codings(content_encoding: &str) -> Vec<&str> {
    content_encoding
        .split(',')
        .map(str::trim)
        .filter(|coding| !coding.is_empty() && *coding != "identity")
        .collect()
}

fn is_single_supported(coding: &str) -> bool {
    matches!(
        coding,
        "gzip" | "x-gzip" | "deflate" | "br" | "zstd" | "zst"
    )
}

fn decompress_single(coding: &str, body: &[u8]) -> Result<Option<Vec<u8>>, std::io::Error> {
    match coding {
        "gzip" | "x-gzip" => {
            let mut decoder = flate2::read::GzDecoder::new(body);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(Some(decompressed))
        }
        "deflate" => {
            let mut decompressed = Vec::new();
            let mut zlib = flate2::read::ZlibDecoder::new(body);
            match zlib.read_to_end(&mut decompressed) {
                Ok(_) => Ok(Some(decompressed)),
                Err(zlib_err) => {
                    log::debug!("deflate zlib decode failed ({zlib_err}); trying raw deflate");
                    let mut decompressed = Vec::new();
                    let mut raw = flate2::read::DeflateDecoder::new(body);
                    raw.read_to_end(&mut decompressed)?;
                    Ok(Some(decompressed))
                }
            }
        }
        "br" => {
            let mut decompressed = Vec::new();
            brotli::BrotliDecompress(&mut std::io::Cursor::new(body), &mut decompressed)?;
            Ok(Some(decompressed))
        }
        "zstd" | "zst" => {
            let decompressed = zstd::stream::decode_all(std::io::Cursor::new(body))?;
            Ok(Some(decompressed))
        }
        _ => Ok(None),
    }
}

pub(crate) fn decompress_body(
    content_encoding: &str,
    body: &[u8],
) -> Result<Option<Vec<u8>>, std::io::Error> {
    let codings = split_codings(content_encoding);
    if codings.is_empty() {
        return Ok(None);
    }

    if !codings.iter().all(|coding| is_single_supported(coding)) {
        log::warn!("unsupported content-encoding: {content_encoding}; skip decompression");
        return Ok(None);
    }

    let mut data: Option<Vec<u8>> = None;
    for coding in codings.iter().rev() {
        let input = data.as_deref().unwrap_or(body);
        match decompress_single(coding, input)? {
            Some(decompressed) => data = Some(decompressed),
            None => return Ok(None),
        }
    }
    Ok(data)
}

pub(crate) fn is_supported_content_encoding(content_encoding: &str) -> bool {
    let codings = split_codings(content_encoding);
    !codings.is_empty() && codings.iter().all(|coding| is_single_supported(coding))
}

pub(crate) fn get_content_encoding(headers: &HeaderMap) -> Option<String> {
    let combined = headers
        .get_all(axum::http::header::CONTENT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
        .to_lowercase();
    if split_codings(&combined).is_empty() {
        return None;
    }
    Some(combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn decompress_body_deflate_handles_zlib_wrapped_per_rfc9110() {
        let payload = br#"{"ok":true}"#;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress_body("deflate", &compressed).unwrap().unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn decompress_body_deflate_falls_back_to_raw_stream() {
        let payload = br#"{"ok":true}"#;
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress_body("deflate", &compressed).unwrap().unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn decompress_body_zstd_roundtrip() {
        let payload = br#"{"hello":"world","n":42}"#;
        let compressed = zstd::stream::encode_all(std::io::Cursor::new(&payload[..]), 0).unwrap();
        let decompressed = decompress_body("zstd", &compressed).unwrap().unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn decompress_body_stacked_gzip_then_zstd_decodes_in_reverse() {
        let payload = br#"{"stacked":true}"#;
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gzip, payload).unwrap();
        let gzipped = gzip.finish().unwrap();
        let stacked = zstd::stream::encode_all(std::io::Cursor::new(&gzipped[..]), 0).unwrap();

        let decompressed = decompress_body("gzip, zstd", &stacked).unwrap().unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn decompress_body_stacked_with_unsupported_returns_none() {
        let result = decompress_body("snappy, zstd", b"\x00\x01\x02\x03").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn decompress_body_unknown_encoding_returns_none_to_keep_headers() {
        let result = decompress_body("snappy", b"\x00\x01\x02\x03").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn is_supported_content_encoding_matches_decompressable() {
        for encoding in [
            "gzip",
            "x-gzip",
            "deflate",
            "br",
            "zstd",
            "zst",
            "gzip, zstd",
        ] {
            assert!(is_supported_content_encoding(encoding), "{encoding}");
        }
        for encoding in ["identity", "snappy", "compress", "", "gzip, snappy"] {
            assert!(!is_supported_content_encoding(encoding), "{encoding}");
        }
    }

    #[test]
    fn get_content_encoding_combines_repeated_headers() {
        let mut headers = HeaderMap::new();
        headers.append(
            axum::http::header::CONTENT_ENCODING,
            HeaderValue::from_static("gzip"),
        );
        headers.append(
            axum::http::header::CONTENT_ENCODING,
            HeaderValue::from_static("zstd"),
        );
        assert_eq!(
            get_content_encoding(&headers).as_deref(),
            Some("gzip, zstd")
        );
    }

    #[test]
    fn get_content_encoding_ignores_identity_only() {
        let mut headers = HeaderMap::new();
        headers.append(
            axum::http::header::CONTENT_ENCODING,
            HeaderValue::from_static("identity"),
        );
        assert_eq!(get_content_encoding(&headers), None);
    }
}
