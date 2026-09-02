use std::{env, time::Duration};

use reqwest::{Client, Url};
use serde::{Deserialize, Deserializer, de::Error as _};
use sha2::{Digest, Sha256};
use thiserror::Error;

const DEFAULT_API_URL: &str = "https://api.packlink.com/v1/";
const DEFAULT_ORIGIN_COUNTRY: &str = "PT";
const DEFAULT_ORIGIN_POSTAL_CODE: &str = "3780-294";
const DEFAULT_PACKAGE_WIDTH_CM: u16 = 35;
const DEFAULT_PACKAGE_LENGTH_CM: u16 = 50;
const DEFAULT_PACKAGE_HEIGHT_CM: u16 = 25;
const DEFAULT_PACKAGE_WEIGHT_GRAMS: u32 = 500;
const MAX_PACKAGES_PER_SHIPMENT: usize = 100;

#[derive(Clone, Debug)]
pub struct PacklinkService {
    client: Client,
    configuration: Option<PacklinkConfiguration>,
}

#[derive(Clone, Debug)]
struct PacklinkConfiguration {
    api_key: String,
    api_url: Url,
    origin_country: String,
    origin_postal_code: String,
    package_width_cm: u16,
    package_length_cm: u16,
    package_height_cm: u16,
    package_weight_grams: u32,
    source: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PacklinkConfigurationStatus {
    pub status: String,
    pub origin: String,
    pub package: String,
}

#[derive(Clone, Debug)]
pub struct PacklinkQuote {
    pub service_id: String,
    pub carrier_name: String,
    pub service_name: String,
    pub amount_minor: i64,
    pub currency: String,
    pub departure_dropoff: bool,
    pub destination_dropoff: bool,
    pub transit_hours: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacklinkPackage {
    pub width_cm: u16,
    pub length_cm: u16,
    pub height_cm: u16,
    pub weight_grams: u32,
}

#[derive(Clone, Debug)]
pub struct PackageItem {
    pub quantity: i64,
    pub unit_weight_grams: i32,
    pub width_cm: i32,
    pub length_cm: i32,
    pub height_cm: i32,
    pub empty_weight_grams: i32,
    pub units_per_package: i32,
}

pub fn packages_for_items(items: &[PackageItem]) -> Option<Vec<PacklinkPackage>> {
    struct PackageBin {
        package: PacklinkPackage,
        empty_weight_grams: i32,
        fill: f64,
    }

    let mut bins = Vec::<PackageBin>::new();
    for item in items {
        if item.quantity <= 0
            || !(1..=1_000_000).contains(&item.unit_weight_grams)
            || !(1..=300).contains(&item.width_cm)
            || !(1..=300).contains(&item.length_cm)
            || !(1..=300).contains(&item.height_cm)
            || !(0..=100_000).contains(&item.empty_weight_grams)
            || !(1..=100).contains(&item.units_per_package)
        {
            return None;
        }
        let width_cm = u16::try_from(item.width_cm).ok()?;
        let length_cm = u16::try_from(item.length_cm).ok()?;
        let height_cm = u16::try_from(item.height_cm).ok()?;
        let unit_weight_grams = u32::try_from(item.unit_weight_grams).ok()?;
        let empty_weight_grams = u32::try_from(item.empty_weight_grams).ok()?;
        let unit_fill = 1.0 / f64::from(item.units_per_package);
        for _ in 0..item.quantity {
            let matching = bins.iter_mut().find(|bin| {
                bin.package.width_cm == width_cm
                    && bin.package.length_cm == length_cm
                    && bin.package.height_cm == height_cm
                    && bin.empty_weight_grams == item.empty_weight_grams
                    && bin.fill + unit_fill <= 1.0 + f64::EPSILON * 16.0
            });
            if let Some(bin) = matching {
                bin.package.weight_grams =
                    bin.package.weight_grams.checked_add(unit_weight_grams)?;
                bin.fill += unit_fill;
                continue;
            }
            if bins.len() >= MAX_PACKAGES_PER_SHIPMENT {
                return None;
            }
            bins.push(PackageBin {
                package: PacklinkPackage {
                    width_cm,
                    length_cm,
                    height_cm,
                    weight_grams: empty_weight_grams.checked_add(unit_weight_grams)?,
                },
                empty_weight_grams: item.empty_weight_grams,
                fill: unit_fill,
            });
        }
    }
    let packages = bins.into_iter().map(|bin| bin.package).collect::<Vec<_>>();
    (!packages.is_empty()).then_some(packages)
}

#[derive(Debug, Error)]
pub enum PacklinkError {
    #[error("Packlink is not configured")]
    NotConfigured,
    #[error("Packlink request could not be completed")]
    Unavailable,
    #[error("Packlink credentials were rejected")]
    Unauthorized,
    #[error("Packlink returned an invalid response")]
    InvalidResponse,
}

#[derive(Deserialize)]
struct ServiceResponse {
    id: serde_json::Value,
    carrier_name: String,
    name: String,
    currency: String,
    #[serde(default)]
    dropoff: bool,
    #[serde(default)]
    delivery_to_parcelshop: bool,
    #[serde(default, deserialize_with = "deserialize_transit_hours")]
    transit_hours: i32,
    price: PriceResponse,
}

#[derive(Deserialize)]
struct PriceResponse {
    total_price: f64,
}

fn deserialize_transit_hours<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(0),
        serde_json::Value::Number(value) => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| D::Error::custom("transit_hours must fit in an integer")),
        serde_json::Value::String(value) => value
            .trim()
            .parse::<i32>()
            .map_err(|_| D::Error::custom("transit_hours must be an integer string")),
        _ => Err(D::Error::custom(
            "transit_hours must be an integer or integer string",
        )),
    }
}

impl Default for PacklinkService {
    fn default() -> Self {
        Self::disabled()
    }
}

impl PacklinkService {
    pub fn from_env() -> Result<Self, String> {
        let Some(api_key) = env::var("PACKLINK_API_KEY")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        else {
            return Ok(Self::disabled());
        };
        let api_url = env::var("PACKLINK_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_URL.into())
            .parse::<Url>()
            .map_err(|_| "PACKLINK_API_URL must be an absolute HTTP(S) URL")?;
        if !matches!(api_url.scheme(), "http" | "https") {
            return Err("PACKLINK_API_URL must be an absolute HTTP(S) URL".into());
        }
        let origin_country = country("PACKLINK_ORIGIN_COUNTRY", DEFAULT_ORIGIN_COUNTRY)?;
        let origin_postal_code = text(
            "PACKLINK_ORIGIN_POSTAL_CODE",
            DEFAULT_ORIGIN_POSTAL_CODE,
            2,
            20,
        )?;
        let package_width_cm = number(
            "PACKLINK_PACKAGE_WIDTH_CM",
            DEFAULT_PACKAGE_WIDTH_CM,
            1,
            300,
        )?;
        let package_length_cm = number(
            "PACKLINK_PACKAGE_LENGTH_CM",
            DEFAULT_PACKAGE_LENGTH_CM,
            1,
            300,
        )?;
        let package_height_cm = number(
            "PACKLINK_PACKAGE_HEIGHT_CM",
            DEFAULT_PACKAGE_HEIGHT_CM,
            1,
            300,
        )?;
        let package_weight_grams = number(
            "PACKLINK_PACKAGE_WEIGHT_GRAMS",
            DEFAULT_PACKAGE_WEIGHT_GRAMS,
            1,
            1_000_000,
        )?;
        let source = env::var("PACKLINK_SOURCE")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .build()
            .map_err(|_| "Packlink HTTP client could not be created")?;
        Ok(Self {
            client,
            configuration: Some(PacklinkConfiguration {
                api_key,
                api_url,
                origin_country,
                origin_postal_code,
                package_width_cm,
                package_length_cm,
                package_height_cm,
                package_weight_grams,
                source,
            }),
        })
    }

    pub fn disabled() -> Self {
        Self {
            client: Client::new(),
            configuration: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.configuration.is_some()
    }

    pub fn status(&self) -> PacklinkConfigurationStatus {
        let Some(configuration) = &self.configuration else {
            return PacklinkConfigurationStatus {
                status: "not_configured".into(),
                origin: format!("{DEFAULT_ORIGIN_POSTAL_CODE}, {DEFAULT_ORIGIN_COUNTRY}"),
                package: format!(
                    "{DEFAULT_PACKAGE_WIDTH_CM} × {DEFAULT_PACKAGE_LENGTH_CM} × {DEFAULT_PACKAGE_HEIGHT_CM} cm · {} kg",
                    DEFAULT_PACKAGE_WEIGHT_GRAMS as f64 / 1000.0
                ),
            };
        };
        PacklinkConfigurationStatus {
            status: "configured".into(),
            origin: format!(
                "{}, {}",
                configuration.origin_postal_code, configuration.origin_country
            ),
            package: format!(
                "{} × {} × {} cm · {} kg",
                configuration.package_width_cm,
                configuration.package_length_cm,
                configuration.package_height_cm,
                configuration.package_weight_grams as f64 / 1000.0
            ),
        }
    }

    pub fn request_hash(
        &self,
        destination_country: &str,
        destination_postal_code: &str,
        packages: &[PacklinkPackage],
    ) -> [u8; 32] {
        let Some(configuration) = &self.configuration else {
            return Sha256::digest(b"packlink-disabled").into();
        };
        let mut signature = format!(
            "{}|{}|{}|{}",
            configuration.origin_country,
            configuration.origin_postal_code,
            destination_country.trim().to_ascii_uppercase(),
            destination_postal_code.trim().to_ascii_uppercase(),
        );
        for package in packages {
            use std::fmt::Write as _;
            let _ = write!(
                signature,
                "|{}x{}x{}:{}",
                package.width_cm, package.length_cm, package.height_cm, package.weight_grams
            );
        }
        Sha256::digest(signature.as_bytes()).into()
    }

    pub async fn quotes(
        &self,
        destination_country: &str,
        destination_postal_code: &str,
        packages: &[PacklinkPackage],
    ) -> Result<Vec<PacklinkQuote>, PacklinkError> {
        let configuration = self
            .configuration
            .as_ref()
            .ok_or(PacklinkError::NotConfigured)?;
        let endpoint = configuration
            .api_url
            .join("services")
            .map_err(|_| PacklinkError::Unavailable)?;
        if packages.is_empty() || packages.len() > MAX_PACKAGES_PER_SHIPMENT {
            return Err(PacklinkError::InvalidResponse);
        }
        let mut query = vec![
            (
                "from[country]".to_owned(),
                configuration.origin_country.clone(),
            ),
            (
                "from[zip]".to_owned(),
                configuration.origin_postal_code.clone(),
            ),
            (
                "to[country]".to_owned(),
                destination_country.trim().to_ascii_uppercase(),
            ),
            (
                "to[zip]".to_owned(),
                destination_postal_code.trim().to_owned(),
            ),
        ];
        for (index, package) in packages.iter().enumerate() {
            query.push((
                format!("packages[{index}][width]"),
                package.width_cm.to_string(),
            ));
            query.push((
                format!("packages[{index}][length]"),
                package.length_cm.to_string(),
            ));
            query.push((
                format!("packages[{index}][height]"),
                package.height_cm.to_string(),
            ));
            query.push((
                format!("packages[{index}][weight]"),
                format!("{:.2}", package.weight_grams as f64 / 1000.0),
            ));
        }
        if let Some(source) = &configuration.source {
            query.push(("source".to_owned(), source.clone()));
        }
        let response = self
            .client
            .get(endpoint)
            .header(reqwest::header::AUTHORIZATION, &configuration.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&query)
            .send()
            .await
            .map_err(|_| PacklinkError::Unavailable)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(PacklinkError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(PacklinkError::Unavailable);
        }
        let services = response
            .json::<Vec<ServiceResponse>>()
            .await
            .map_err(|_| PacklinkError::InvalidResponse)?;
        let mut quotes = services
            .into_iter()
            .filter(|service| !service.delivery_to_parcelshop)
            .filter_map(|service| {
                let service_id = match service.id {
                    serde_json::Value::String(value) => value,
                    serde_json::Value::Number(value) => value.to_string(),
                    _ => return None,
                };
                let amount_minor = (service.price.total_price * 100.0).round();
                if !amount_minor.is_finite()
                    || amount_minor < 0.0
                    || service.carrier_name.trim().is_empty()
                    || service.name.trim().is_empty()
                    || service.currency.len() != 3
                {
                    return None;
                }
                Some(PacklinkQuote {
                    service_id,
                    carrier_name: service.carrier_name.trim().to_owned(),
                    service_name: service.name.trim().to_owned(),
                    amount_minor: amount_minor as i64,
                    currency: service.currency.to_ascii_uppercase(),
                    departure_dropoff: service.dropoff,
                    destination_dropoff: service.delivery_to_parcelshop,
                    transit_hours: service.transit_hours.clamp(0, 8760),
                })
            })
            .collect::<Vec<_>>();
        quotes.sort_by_key(|quote| (quote.amount_minor, quote.transit_hours));
        quotes.dedup_by(|left, right| {
            left.service_id == right.service_id
                && left.departure_dropoff == right.departure_dropoff
                && left.destination_dropoff == right.destination_dropoff
        });
        Ok(customer_quote_choices(quotes))
    }
}

fn customer_quote_choices(quotes: Vec<PacklinkQuote>) -> Vec<PacklinkQuote> {
    let Some(cheapest) = quotes.first().cloned() else {
        return Vec::new();
    };
    let fastest = quotes
        .iter()
        .filter(|quote| quote.transit_hours > 0)
        .min_by_key(|quote| (quote.transit_hours, quote.amount_minor))
        .cloned();
    let mut choices = vec![cheapest];
    if let Some(fastest) = fastest
        && !choices.iter().any(|choice| {
            choice.service_id == fastest.service_id
                && choice.departure_dropoff == fastest.departure_dropoff
                && choice.destination_dropoff == fastest.destination_dropoff
        })
    {
        choices.push(fastest);
    }
    choices
}

pub fn configured_in_environment() -> bool {
    env::var("PACKLINK_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

fn country(name: &str, fallback: &str) -> Result<String, String> {
    let value = env::var(name)
        .unwrap_or_else(|_| fallback.into())
        .trim()
        .to_ascii_uppercase();
    if value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Ok(value)
    } else {
        Err(format!("{name} must be a two-letter country code"))
    }
}

fn text(name: &str, fallback: &str, minimum: usize, maximum: usize) -> Result<String, String> {
    let value = env::var(name)
        .unwrap_or_else(|_| fallback.into())
        .trim()
        .to_owned();
    if (minimum..=maximum).contains(&value.len()) {
        Ok(value)
    } else {
        Err(format!("{name} has an invalid length"))
    }
}

fn number<T>(name: &str, fallback: T, minimum: T, maximum: T) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + Copy,
{
    let value = match env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|_| format!("{name} must be a number"))?,
        Err(_) => fallback,
    };
    if value < minimum || value > maximum {
        Err(format!("{name} is outside the supported range"))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::{Client, Url};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::{
        PackageItem, PacklinkConfiguration, PacklinkPackage, PacklinkService, ServiceResponse,
        packages_for_items,
    };

    #[test]
    fn cart_quantities_are_split_into_configured_parcels() {
        let packages = packages_for_items(&[PackageItem {
            quantity: 5,
            unit_weight_grams: 700,
            width_cm: 40,
            length_cm: 50,
            height_cm: 20,
            empty_weight_grams: 0,
            units_per_package: 2,
        }])
        .unwrap();
        assert_eq!(
            packages,
            vec![
                PacklinkPackage {
                    width_cm: 40,
                    length_cm: 50,
                    height_cm: 20,
                    weight_grams: 1400,
                },
                PacklinkPackage {
                    width_cm: 40,
                    length_cm: 50,
                    height_cm: 20,
                    weight_grams: 1400,
                },
                PacklinkPackage {
                    width_cm: 40,
                    length_cm: 50,
                    height_cm: 20,
                    weight_grams: 700,
                },
            ]
        );
    }

    #[test]
    fn products_with_the_same_package_share_available_capacity() {
        let packages = packages_for_items(&[
            PackageItem {
                quantity: 1,
                unit_weight_grams: 700,
                width_cm: 40,
                length_cm: 50,
                height_cm: 20,
                empty_weight_grams: 100,
                units_per_package: 2,
            },
            PackageItem {
                quantity: 5,
                unit_weight_grams: 100,
                width_cm: 40,
                length_cm: 50,
                height_cm: 20,
                empty_weight_grams: 100,
                units_per_package: 10,
            },
        ])
        .unwrap();

        assert_eq!(
            packages,
            vec![PacklinkPackage {
                width_cm: 40,
                length_cm: 50,
                height_cm: 20,
                weight_grams: 1300,
            }]
        );
    }

    #[test]
    fn service_response_matches_packlink_price_shape() {
        let response: ServiceResponse = serde_json::from_value(serde_json::json!({
            "id": 20345,
            "carrier_name": "Transportadora",
            "name": "Entrega 24 h",
            "currency": "EUR",
            "dropoff": true,
            "delivery_to_parcelshop": false,
            "transit_hours": "24",
            "price": { "total_price": 4.93 }
        }))
        .unwrap();
        assert_eq!(response.carrier_name, "Transportadora");
        assert_eq!(response.price.total_price, 4.93);
        assert_eq!(response.transit_hours, 24);
    }

    #[test]
    fn disabled_service_is_explicit() {
        let service = PacklinkService::disabled();
        assert!(!service.enabled());
        assert_eq!(service.status().status, "not_configured");
    }

    #[tokio::test]
    async fn quotes_use_server_side_credentials_and_hide_recipient_parcelshops() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 8192];
            let read = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..read]).into_owned();
            let body = serde_json::json!([
                {
                    "id": 21,
                    "carrier_name": "Carrier A",
                    "name": "Home delivery",
                    "currency": "EUR",
                    "dropoff": false,
                    "delivery_to_parcelshop": false,
                    "transit_hours": 24,
                    "price": { "total_price": 4.93 }
                },
                {
                    "id": 22,
                    "carrier_name": "Carrier B",
                    "name": "Recipient pickup point",
                    "currency": "EUR",
                    "dropoff": true,
                    "delivery_to_parcelshop": true,
                    "transit_hours": 48,
                    "price": { "total_price": 3.75 }
                },
                {
                    "id": 23,
                    "carrier_name": "Carrier C",
                    "name": "Fast delivery",
                    "currency": "EUR",
                    "dropoff": false,
                    "delivery_to_parcelshop": false,
                    "transit_hours": "12",
                    "price": { "total_price": 6.25 }
                },
                {
                    "id": 24,
                    "carrier_name": "Carrier D",
                    "name": "Middle option",
                    "currency": "EUR",
                    "dropoff": false,
                    "delivery_to_parcelshop": false,
                    "transit_hours": 36,
                    "price": { "total_price": 5.20 }
                }
            ])
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            request
        });
        let service = PacklinkService {
            client: Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
            configuration: Some(PacklinkConfiguration {
                api_key: "server-secret-key".into(),
                api_url: Url::parse(&format!("http://{address}/v1/")).unwrap(),
                origin_country: "PT".into(),
                origin_postal_code: "3780-294".into(),
                package_width_cm: 35,
                package_length_cm: 50,
                package_height_cm: 25,
                package_weight_grams: 500,
                source: None,
            }),
        };

        let packages = vec![
            PacklinkPackage {
                width_cm: 35,
                length_cm: 50,
                height_cm: 25,
                weight_grams: 500,
            },
            PacklinkPackage {
                width_cm: 45,
                length_cm: 60,
                height_cm: 30,
                weight_grams: 1_250,
            },
        ];
        let quotes = service.quotes("PT", "1000-001", &packages).await.unwrap();
        let request = server.await.unwrap();

        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].service_id, "21");
        assert_eq!(quotes[0].amount_minor, 493);
        assert_eq!(quotes[1].service_id, "23");
        assert_eq!(quotes[1].transit_hours, 12);
        assert!(request.contains("authorization: server-secret-key\r\n"));
        assert!(request.contains("from%5Bzip%5D=3780-294"));
        assert!(request.contains("to%5Bzip%5D=1000-001"));
        assert!(request.contains("packages%5B0%5D%5Bweight%5D=0.50"));
        assert!(request.contains("packages%5B1%5D%5Bweight%5D=1.25"));
    }
}
