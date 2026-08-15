use std::{env, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

const CHUNK_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024;

#[derive(Clone, Debug, Default)]
pub enum MediaScanner {
    #[default]
    Disabled,
    ClamAv {
        address: Arc<str>,
        timeout: Duration,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum ScanOutcome {
    Clean,
    Infected(String),
}

impl MediaScanner {
    pub fn from_env(production: bool) -> Result<Self, String> {
        Self::from_values(
            production,
            env::var("MEDIA_SCANNER_ADDRESS").ok(),
            env::var("MEDIA_SCAN_TIMEOUT_SECONDS").ok(),
        )
    }

    fn from_values(
        production: bool,
        address: Option<String>,
        timeout_seconds: Option<String>,
    ) -> Result<Self, String> {
        let timeout_seconds = timeout_seconds
            .map_or(Some(10), |value| value.parse::<u64>().ok())
            .filter(|value| (1..=60).contains(value))
            .ok_or_else(|| "MEDIA_SCAN_TIMEOUT_SECONDS must be between 1 and 60".to_owned())?;
        match address
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(address) if address.len() <= 255 && address.contains(':') => Ok(Self::ClamAv {
                address: Arc::from(address),
                timeout: Duration::from_secs(timeout_seconds),
            }),
            Some(_) => Err("MEDIA_SCANNER_ADDRESS must be a host:port value".into()),
            None if production => {
                Err("MEDIA_SCANNER_ADDRESS is required in production so uploads fail closed".into())
            }
            None => Ok(Self::Disabled),
        }
    }

    pub async fn scan(&self, bytes: &[u8]) -> Result<ScanOutcome, String> {
        let Self::ClamAv {
            address,
            timeout: scan_timeout,
        } = self
        else {
            return Ok(ScanOutcome::Clean);
        };
        timeout(*scan_timeout, scan_clamav(address, bytes))
            .await
            .map_err(|_| "malware scanner timed out".to_owned())?
    }
}

async fn scan_clamav(address: &str, bytes: &[u8]) -> Result<ScanOutcome, String> {
    let mut stream = TcpStream::connect(address)
        .await
        .map_err(|_| "malware scanner is unavailable".to_owned())?;
    stream
        .write_all(b"zINSTREAM\0")
        .await
        .map_err(|_| "malware scanner request failed".to_owned())?;
    for chunk in bytes.chunks(CHUNK_BYTES) {
        stream
            .write_all(&(chunk.len() as u32).to_be_bytes())
            .await
            .map_err(|_| "malware scanner request failed".to_owned())?;
        stream
            .write_all(chunk)
            .await
            .map_err(|_| "malware scanner request failed".to_owned())?;
    }
    stream
        .write_all(&0_u32.to_be_bytes())
        .await
        .map_err(|_| "malware scanner request failed".to_owned())?;
    stream
        .flush()
        .await
        .map_err(|_| "malware scanner request failed".to_owned())?;

    let mut response = Vec::new();
    BufReader::new(stream)
        .take(MAX_RESPONSE_BYTES as u64)
        .read_until(0, &mut response)
        .await
        .map_err(|_| "malware scanner response failed".to_owned())?;
    let response = String::from_utf8_lossy(&response);
    let response = response.trim_end_matches(['\0', '\r', '\n']);
    parse_response(response)
}

fn parse_response(response: &str) -> Result<ScanOutcome, String> {
    if response.ends_with(" OK") {
        return Ok(ScanOutcome::Clean);
    }
    if let Some(signature) = response
        .strip_suffix(" FOUND")
        .and_then(|value| {
            value
                .rsplit_once(':')
                .map(|(_, signature)| signature.trim())
        })
        .filter(|signature| !signature.is_empty())
    {
        return Ok(ScanOutcome::Infected(signature.to_owned()));
    }
    Err("malware scanner returned an invalid or error response".into())
}

#[cfg(test)]
mod tests {
    use super::{MediaScanner, ScanOutcome, parse_response};

    #[tokio::test]
    async fn disabled_scanner_is_explicitly_clean_outside_production() {
        assert_eq!(
            MediaScanner::Disabled.scan(b"safe image").await,
            Ok(ScanOutcome::Clean)
        );
    }

    #[test]
    fn clamav_signature_is_reported_without_accepting_the_file() {
        assert_eq!(
            parse_response("stream: Eicar-Test-Signature FOUND"),
            Ok(ScanOutcome::Infected("Eicar-Test-Signature".into()))
        );
    }

    #[test]
    fn clamav_clean_and_error_responses_are_distinct() {
        assert_eq!(parse_response("stream: OK"), Ok(ScanOutcome::Clean));
        assert!(parse_response("stream: scanner unavailable ERROR").is_err());
    }

    #[test]
    fn production_requires_a_bounded_scanner_configuration() {
        assert!(MediaScanner::from_values(true, None, None).is_err());
        assert!(
            MediaScanner::from_values(true, Some("clamav.internal:3310".into()), Some("0".into()))
                .is_err()
        );
        assert!(matches!(
            MediaScanner::from_values(true, Some("clamav.internal:3310".into()), Some("10".into())),
            Ok(MediaScanner::ClamAv { .. })
        ));
    }
}
