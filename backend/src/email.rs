use std::{env, sync::Arc};

use aws_sdk_sesv2::{
    Client,
    types::{Body, Content, Destination, EmailContent, Message},
};
use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;
use utoipa::ToSchema;

use crate::{AppState, config::Environment, customers::valid_email, error::ErrorBody};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountEmailKind {
    Verification,
    PasswordReset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderEmailKind {
    Confirmation,
    Fulfillment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EmailDeliveryMode {
    Development,
    Ses,
}

impl OrderEmailKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmation => "order_confirmation",
            Self::Fulfillment => "fulfillment_created",
        }
    }

    const fn subject(self) -> &'static str {
        match self {
            Self::Confirmation => "Your KnitPrint order is confirmed",
            Self::Fulfillment => "Your KnitPrint order is on its way",
        }
    }
}

pub struct OrderEmail<'a> {
    pub to: &'a str,
    pub first_name: &'a str,
    pub kind: OrderEmailKind,
    pub order_number: &'a str,
    pub total: &'a str,
    pub carrier: &'a str,
    pub tracking_number: &'a str,
    pub tracking_url: &'a str,
}

impl AccountEmailKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verification => "email_verification",
            Self::PasswordReset => "password_reset",
        }
    }

    fn query_name(self) -> &'static str {
        match self {
            Self::Verification => "verify",
            Self::PasswordReset => "reset",
        }
    }

    fn subject(self) -> &'static str {
        match self {
            Self::Verification => "Verify your KnitPrint email",
            Self::PasswordReset => "Reset your KnitPrint password",
        }
    }

    fn action_label(self) -> &'static str {
        match self {
            Self::Verification => "Verify email address",
            Self::PasswordReset => "Reset password",
        }
    }

    fn expiry_description(self) -> &'static str {
        match self {
            Self::Verification => "24 hours",
            Self::PasswordReset => "one hour",
        }
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct DevelopmentEmail {
    pub to: String,
    pub kind: String,
    pub subject: String,
    pub action_url: String,
}

#[derive(Clone)]
enum Delivery {
    Development(Arc<RwLock<Vec<DevelopmentEmail>>>),
    Ses {
        client: Client,
        from: String,
        configuration_set: Option<String>,
    },
    Disabled,
}

#[derive(Clone)]
pub struct EmailService {
    delivery: Delivery,
    storefront_base_url: String,
}

impl Default for EmailService {
    fn default() -> Self {
        Self {
            delivery: Delivery::Disabled,
            storefront_base_url: "http://127.0.0.1:3000".into(),
        }
    }
}

impl EmailService {
    pub fn status(&self) -> &'static str {
        match &self.delivery {
            Delivery::Development(_) => "development_mailbox",
            Delivery::Ses { .. } => "ses_configured",
            Delivery::Disabled => "unavailable",
        }
    }

    pub async fn from_env(environment: Environment) -> Result<Self, String> {
        let base_url =
            env::var("STOREFRONT_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
        validate_base_url(&base_url, environment == Environment::Production)?;
        let storefront_base_url = base_url.trim_end_matches('/').to_owned();

        if delivery_mode(environment, env::var("EMAIL_DELIVERY").ok().as_deref())?
            == EmailDeliveryMode::Development
        {
            return Ok(Self::development(storefront_base_url));
        }

        let from =
            env::var("EMAIL_FROM").map_err(|_| "EMAIL_FROM is required when EMAIL_DELIVERY=ses")?;
        if !valid_email(&from) {
            return Err("EMAIL_FROM must be a valid email address".into());
        }
        let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(Self {
            delivery: Delivery::Ses {
                client: Client::new(&sdk_config),
                from,
                configuration_set: env::var("SES_CONFIGURATION_SET").ok(),
            },
            storefront_base_url,
        })
    }

    pub fn development(base_url: impl Into<String>) -> Self {
        Self {
            delivery: Delivery::Development(Arc::new(RwLock::new(Vec::new()))),
            storefront_base_url: base_url.into().trim_end_matches('/').to_owned(),
        }
    }

    pub async fn send_account_action(
        &self,
        to: &str,
        first_name: &str,
        kind: AccountEmailKind,
        token: &str,
    ) -> Result<(), String> {
        let action_url = format!(
            "{}/account?{}={token}",
            self.storefront_base_url,
            kind.query_name()
        );
        match &self.delivery {
            Delivery::Development(mailbox) => {
                mailbox.write().await.push(DevelopmentEmail {
                    to: to.into(),
                    kind: kind.as_str().into(),
                    subject: kind.subject().into(),
                    action_url,
                });
                Ok(())
            }
            Delivery::Ses {
                client,
                from,
                configuration_set,
            } => {
                let safe_name = escape_html(first_name);
                let text = format!(
                    "Hello {first_name},\n\n{}: {action_url}\n\nThis link expires in {}. If you did not request this, you can ignore this email.\n",
                    kind.action_label(),
                    kind.expiry_description(),
                );
                let html = format!(
                    "<p>Hello {safe_name},</p><p><a href=\"{action_url}\">{}</a></p><p>This link expires in {}. If you did not request this, you can ignore this email.</p>",
                    kind.action_label(),
                    kind.expiry_description(),
                );
                let subject = Content::builder()
                    .data(kind.subject())
                    .charset("UTF-8")
                    .build()
                    .map_err(|error| format!("email subject is invalid: {error}"))?;
                let text = Content::builder()
                    .data(text)
                    .charset("UTF-8")
                    .build()
                    .map_err(|error| format!("email text is invalid: {error}"))?;
                let html = Content::builder()
                    .data(html)
                    .charset("UTF-8")
                    .build()
                    .map_err(|error| format!("email HTML is invalid: {error}"))?;
                let content = EmailContent::builder()
                    .simple(
                        Message::builder()
                            .subject(subject)
                            .body(Body::builder().text(text).html(html).build())
                            .build(),
                    )
                    .build();
                let mut request = client
                    .send_email()
                    .from_email_address(from)
                    .destination(Destination::builder().to_addresses(to).build())
                    .content(content);
                if let Some(configuration_set) = configuration_set {
                    request = request.configuration_set_name(configuration_set);
                }
                request
                    .send()
                    .await
                    .map_err(|error| format!("SES send failed: {error}"))?;
                Ok(())
            }
            Delivery::Disabled => Err("email delivery is not configured".into()),
        }
    }

    pub async fn send_order_notification(&self, email: OrderEmail<'_>) -> Result<(), String> {
        let order_url = format!("{}/cart", self.storefront_base_url);
        let action_url = if email.tracking_url.is_empty() {
            order_url
        } else {
            email.tracking_url.to_owned()
        };
        match &self.delivery {
            Delivery::Development(mailbox) => {
                mailbox.write().await.push(DevelopmentEmail {
                    to: email.to.into(),
                    kind: email.kind.as_str().into(),
                    subject: email.kind.subject().into(),
                    action_url,
                });
                Ok(())
            }
            Delivery::Ses {
                client,
                from,
                configuration_set,
            } => {
                let safe_name = escape_html(email.first_name);
                let safe_order = escape_html(email.order_number);
                let safe_total = escape_html(email.total);
                let (text, html) = match email.kind {
                    OrderEmailKind::Confirmation => (
                        format!(
                            "Hello {},\n\nYour KnitPrint order {} is confirmed. Total: {}.\n\nWe will email you again when it ships.\n",
                            email.first_name, email.order_number, email.total
                        ),
                        format!(
                            "<p>Hello {safe_name},</p><p>Your KnitPrint order <strong>{safe_order}</strong> is confirmed.</p><p>Total: {safe_total}</p><p>We will email you again when it ships.</p>"
                        ),
                    ),
                    OrderEmailKind::Fulfillment => {
                        let tracking_text = if email.tracking_number.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " Carrier: {}. Tracking: {}.",
                                email.carrier, email.tracking_number
                            )
                        };
                        let tracking_html = if email.tracking_number.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "<p>{}: <a href=\"{}\">{}</a></p>",
                                escape_html(email.carrier),
                                escape_html(&action_url),
                                escape_html(email.tracking_number)
                            )
                        };
                        (
                            format!(
                                "Hello {},\n\nYour KnitPrint order {} has shipped.{}\n",
                                email.first_name, email.order_number, tracking_text
                            ),
                            format!(
                                "<p>Hello {safe_name},</p><p>Your KnitPrint order <strong>{safe_order}</strong> has shipped.</p>{tracking_html}"
                            ),
                        )
                    }
                };
                send_ses(
                    client,
                    from,
                    configuration_set.as_deref(),
                    email.to,
                    email.kind.subject(),
                    text,
                    html,
                )
                .await
            }
            Delivery::Disabled => Err("email delivery is not configured".into()),
        }
    }

    async fn latest_development_email(&self, to: &str, kind: &str) -> Option<DevelopmentEmail> {
        let Delivery::Development(mailbox) = &self.delivery else {
            return None;
        };
        mailbox
            .read()
            .await
            .iter()
            .rev()
            .find(|email| email.to.eq_ignore_ascii_case(to) && email.kind == kind)
            .cloned()
    }
}

async fn send_ses(
    client: &Client,
    from: &str,
    configuration_set: Option<&str>,
    to: &str,
    subject_value: &str,
    text_value: String,
    html_value: String,
) -> Result<(), String> {
    let subject = Content::builder()
        .data(subject_value)
        .charset("UTF-8")
        .build()
        .map_err(|error| format!("email subject is invalid: {error}"))?;
    let text = Content::builder()
        .data(text_value)
        .charset("UTF-8")
        .build()
        .map_err(|error| format!("email text is invalid: {error}"))?;
    let html = Content::builder()
        .data(html_value)
        .charset("UTF-8")
        .build()
        .map_err(|error| format!("email HTML is invalid: {error}"))?;
    let content = EmailContent::builder()
        .simple(
            Message::builder()
                .subject(subject)
                .body(Body::builder().text(text).html(html).build())
                .build(),
        )
        .build();
    let mut request = client
        .send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(content);
    if let Some(configuration_set) = configuration_set {
        request = request.configuration_set_name(configuration_set);
    }
    request
        .send()
        .await
        .map_err(|error| format!("SES send failed: {error}"))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct DevelopmentEmailQuery {
    to: String,
    kind: String,
}

pub async fn development_latest(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(query): Query<DevelopmentEmailQuery>,
) -> impl IntoResponse {
    if let Some(email) = state
        .email
        .latest_development_email(&query.to, &query.kind)
        .await
    {
        return Json(email).into_response();
    }
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody::new(
            "development_email_not_found",
            "No matching development email was found.",
        )),
    )
        .into_response()
}

pub fn log_delivery_failure(error: &str, kind: AccountEmailKind) {
    warn!(%error, email_kind = kind.as_str(), "account email delivery failed");
}

fn delivery_mode(
    environment: Environment,
    configured: Option<&str>,
) -> Result<EmailDeliveryMode, String> {
    match (environment, configured) {
        (Environment::Production, None | Some("ses")) => Ok(EmailDeliveryMode::Ses),
        (Environment::Production, Some("development")) => {
            Err("EMAIL_DELIVERY=development is not allowed in production".into())
        }
        (Environment::Development | Environment::Test, None | Some("development")) => {
            Ok(EmailDeliveryMode::Development)
        }
        (Environment::Development | Environment::Test, Some("ses")) => Ok(EmailDeliveryMode::Ses),
        (_, Some(_)) => Err("EMAIL_DELIVERY must be one of: development, ses".into()),
    }
}

fn validate_base_url(value: &str, production: bool) -> Result<(), String> {
    let value = value.trim();
    let valid_scheme = value.starts_with("http://") || value.starts_with("https://");
    if value.is_empty()
        || value.chars().any(char::is_whitespace)
        || value.chars().any(|character| "<>\"'".contains(character))
        || !valid_scheme
    {
        return Err("STOREFRONT_BASE_URL must be an absolute HTTP(S) URL".into());
    }
    if production && !value.starts_with("https://") {
        return Err("STOREFRONT_BASE_URL must use HTTPS in production".into());
    }
    Ok(())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::{
        AccountEmailKind, EmailDeliveryMode, EmailService, delivery_mode, validate_base_url,
    };
    use crate::config::Environment;

    #[tokio::test]
    async fn development_delivery_keeps_action_links_out_of_logs_and_respects_kinds() {
        let service = EmailService::development("http://127.0.0.1:3000/");
        service
            .send_account_action(
                "marta@example.test",
                "Marta",
                AccountEmailKind::Verification,
                "secret-token",
            )
            .await
            .unwrap();
        let email = service
            .latest_development_email("MARTA@example.test", "email_verification")
            .await
            .unwrap();
        assert_eq!(
            email.action_url,
            "http://127.0.0.1:3000/account?verify=secret-token"
        );
        assert!(
            service
                .latest_development_email("marta@example.test", "password_reset")
                .await
                .is_none()
        );
    }

    #[test]
    fn production_action_urls_require_https() {
        assert!(validate_base_url("https://shop.example.com", true).is_ok());
        assert!(validate_base_url("http://shop.example.com", true).is_err());
        assert!(validate_base_url("shop.example.com", false).is_err());
    }

    #[test]
    fn development_defaults_to_mailbox_and_can_opt_into_ses() {
        assert_eq!(
            delivery_mode(Environment::Development, None).unwrap(),
            EmailDeliveryMode::Development
        );
        assert_eq!(
            delivery_mode(Environment::Development, Some("development")).unwrap(),
            EmailDeliveryMode::Development
        );
        assert_eq!(
            delivery_mode(Environment::Development, Some("ses")).unwrap(),
            EmailDeliveryMode::Ses
        );
    }

    #[test]
    fn production_defaults_to_ses_and_rejects_development_mailbox() {
        assert_eq!(
            delivery_mode(Environment::Production, None).unwrap(),
            EmailDeliveryMode::Ses
        );
        assert_eq!(
            delivery_mode(Environment::Production, Some("ses")).unwrap(),
            EmailDeliveryMode::Ses
        );
        assert!(
            delivery_mode(Environment::Production, Some("development"))
                .unwrap_err()
                .contains("not allowed")
        );
    }

    #[test]
    fn delivery_mode_rejects_unknown_values() {
        assert_eq!(
            delivery_mode(Environment::Test, Some("smtp")).unwrap_err(),
            "EMAIL_DELIVERY must be one of: development, ses"
        );
    }
}
