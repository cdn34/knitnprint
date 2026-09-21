use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, request::Parts},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};

use crate::AppState;

const ACCOUNT_LIMIT: i32 = 5;
const ACCOUNT_WINDOW_MINUTES: i32 = 15;
const ACCOUNT_LOCK_MINUTES: i32 = 15;
const IP_LIMIT: i32 = 60;
const IP_WINDOW_MINUTES: i32 = 5;
const IP_LOCK_MINUTES: i32 = 5;
const GLOBAL_LIMIT: i32 = 1_000;
const GLOBAL_WINDOW_MINUTES: i32 = 1;
const GLOBAL_LOCK_MINUTES: i32 = 1;
const ACTION_ACCOUNT_LIMIT: i32 = 5;
const ACTION_ACCOUNT_WINDOW_MINUTES: i32 = 60;
const ACTION_IP_LIMIT: i32 = 20;
const ACTION_IP_WINDOW_MINUTES: i32 = 15;
const ACTION_GLOBAL_LIMIT: i32 = 300;
const ACTION_GLOBAL_WINDOW_MINUTES: i32 = 1;

#[derive(Clone, Copy)]
pub enum AuthScope {
    Staff,
    Customer,
    AccountAction,
}

impl AuthScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Customer => "customer",
            Self::AccountAction => "account_action",
        }
    }
}

pub async fn consume_account_action(
    pool: &PgPool,
    action: &str,
    account_identifier: &str,
    client_ip: IpAddr,
) -> Result<(), LoginLimitError> {
    let scope = AuthScope::AccountAction;
    let global_hash = bucket_hash(scope, "global", action);
    let ip_hash = bucket_hash(scope, "ip", &format!("{action}\0{client_ip}"));
    let account_hash = bucket_hash(scope, "account", &format!("{action}\0{account_identifier}"));
    let mut transaction = pool.begin().await?;
    for hash in [&global_hash, &ip_hash, &account_hash] {
        advisory_lock(&mut transaction, hash).await?;
    }
    for (dimension, hash, retry_after) in [
        ("global", &global_hash, 60),
        ("ip", &ip_hash, 900),
        ("account", &account_hash, 3600),
    ] {
        if bucket_is_locked(&mut transaction, scope, dimension, hash).await? {
            transaction.rollback().await?;
            return Err(LoginLimitError::Limited(retry_after));
        }
    }
    for (dimension, hash, limit, window) in [
        (
            "global",
            &global_hash,
            ACTION_GLOBAL_LIMIT,
            ACTION_GLOBAL_WINDOW_MINUTES,
        ),
        ("ip", &ip_hash, ACTION_IP_LIMIT, ACTION_IP_WINDOW_MINUTES),
        (
            "account",
            &account_hash,
            ACTION_ACCOUNT_LIMIT,
            ACTION_ACCOUNT_WINDOW_MINUTES,
        ),
    ] {
        record_event(
            &mut transaction,
            scope,
            dimension,
            hash,
            limit,
            window,
            window,
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

#[derive(Debug)]
pub enum LoginLimitError {
    Limited(u64),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for LoginLimitError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

pub struct AccountLoginGuard<'a> {
    transaction: Transaction<'a, Postgres>,
    scope: AuthScope,
    account_hash: [u8; 32],
}

pub struct ClientIp(pub IpAddr);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let connect = parts.extensions.get::<ConnectInfo<SocketAddr>>().copied();
        Ok(Self(resolve_client_ip(
            connect,
            &parts.headers,
            state.trusted_proxy_hops,
        )))
    }
}

impl<'a> AccountLoginGuard<'a> {
    pub async fn acquire(
        pool: &'a PgPool,
        scope: AuthScope,
        account_identifier: &str,
        client_ip: IpAddr,
    ) -> Result<Self, LoginLimitError> {
        consume_request_volume(pool, scope, client_ip).await?;
        let account_hash = bucket_hash(scope, "account", account_identifier);
        let mut transaction = pool.begin().await?;
        advisory_lock(&mut transaction, &account_hash).await?;
        if bucket_is_locked(&mut transaction, scope, "account", &account_hash).await? {
            transaction.rollback().await?;
            return Err(LoginLimitError::Limited(900));
        }
        Ok(Self {
            transaction,
            scope,
            account_hash,
        })
    }

    pub fn transaction(&mut self) -> &mut Transaction<'a, Postgres> {
        &mut self.transaction
    }

    pub async fn record_failure(mut self) -> Result<(), sqlx::Error> {
        record_event(
            &mut self.transaction,
            self.scope,
            "account",
            &self.account_hash,
            ACCOUNT_LIMIT,
            ACCOUNT_WINDOW_MINUTES,
            ACCOUNT_LOCK_MINUTES,
        )
        .await?;
        self.transaction.commit().await
    }

    pub async fn record_success(mut self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM auth_login_rate_limits WHERE auth_scope = $1 AND dimension = 'account' AND key_hash = $2",
        )
        .bind(self.scope.as_str())
        .bind(self.account_hash.as_slice())
        .execute(&mut *self.transaction)
        .await?;
        self.transaction.commit().await
    }
}

async fn consume_request_volume(
    pool: &PgPool,
    scope: AuthScope,
    client_ip: IpAddr,
) -> Result<(), LoginLimitError> {
    let global_hash = bucket_hash(scope, "global", "all");
    let ip_hash = bucket_hash(scope, "ip", &client_ip.to_string());
    let mut transaction = pool.begin().await?;
    advisory_lock(&mut transaction, &global_hash).await?;
    advisory_lock(&mut transaction, &ip_hash).await?;
    if bucket_is_locked(&mut transaction, scope, "global", &global_hash).await? {
        transaction.rollback().await?;
        return Err(LoginLimitError::Limited(60));
    }
    if bucket_is_locked(&mut transaction, scope, "ip", &ip_hash).await? {
        transaction.rollback().await?;
        return Err(LoginLimitError::Limited(300));
    }
    record_event(
        &mut transaction,
        scope,
        "global",
        &global_hash,
        GLOBAL_LIMIT,
        GLOBAL_WINDOW_MINUTES,
        GLOBAL_LOCK_MINUTES,
    )
    .await?;
    record_event(
        &mut transaction,
        scope,
        "ip",
        &ip_hash,
        IP_LIMIT,
        IP_WINDOW_MINUTES,
        IP_LOCK_MINUTES,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn advisory_lock(
    transaction: &mut Transaction<'_, Postgres>,
    hash: &[u8; 32],
) -> Result<(), sqlx::Error> {
    let key = i64::from_be_bytes(
        hash[..8]
            .try_into()
            .expect("SHA-256 always contains eight bytes"),
    );
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn bucket_is_locked(
    transaction: &mut Transaction<'_, Postgres>,
    scope: AuthScope,
    dimension: &str,
    key_hash: &[u8; 32],
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(locked_until > now(), false)
        FROM auth_login_rate_limits
        WHERE auth_scope = $1 AND dimension = $2 AND key_hash = $3
        "#,
    )
    .bind(scope.as_str())
    .bind(dimension)
    .bind(key_hash.as_slice())
    .fetch_optional(&mut **transaction)
    .await
    .map(|locked| locked.unwrap_or(false))
}

#[allow(clippy::too_many_arguments)]
async fn record_event(
    transaction: &mut Transaction<'_, Postgres>,
    scope: AuthScope,
    dimension: &str,
    key_hash: &[u8; 32],
    limit: i32,
    window_minutes: i32,
    lock_minutes: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO auth_login_rate_limits (
            auth_scope, dimension, key_hash, event_count
        ) VALUES ($1, $2, $3, 1)
        ON CONFLICT (auth_scope, dimension, key_hash) DO UPDATE SET
            event_count = CASE
                WHEN auth_login_rate_limits.window_started_at < now() - make_interval(mins => $5)
                    THEN 1
                ELSE auth_login_rate_limits.event_count + 1
            END,
            window_started_at = CASE
                WHEN auth_login_rate_limits.window_started_at < now() - make_interval(mins => $5)
                    THEN now()
                ELSE auth_login_rate_limits.window_started_at
            END,
            locked_until = CASE
                WHEN (CASE
                    WHEN auth_login_rate_limits.window_started_at < now() - make_interval(mins => $5)
                        THEN 1
                    ELSE auth_login_rate_limits.event_count + 1
                END) >= $4 THEN now() + make_interval(mins => $6)
                ELSE NULL
            END,
            updated_at = now()
        "#,
    )
    .bind(scope.as_str())
    .bind(dimension)
    .bind(key_hash.as_slice())
    .bind(limit)
    .bind(window_minutes)
    .bind(lock_minutes)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn bucket_hash(scope: AuthScope, dimension: &str, value: &str) -> [u8; 32] {
    Sha256::digest(format!("{}\0{dimension}\0{value}", scope.as_str()).as_bytes()).into()
}

pub fn resolve_client_ip(
    connect: Option<ConnectInfo<SocketAddr>>,
    headers: &HeaderMap,
    trusted_proxy_hops: usize,
) -> IpAddr {
    if trusted_proxy_hops > 0
        && let Some(forwarded) = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').rev().nth(trusted_proxy_hops - 1))
            .map(str::trim)
            .and_then(parse_forwarded_ip)
    {
        return forwarded;
    }
    connect
        .map(|ConnectInfo(address)| address.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
}

fn parse_forwarded_ip(value: &str) -> Option<IpAddr> {
    value
        .parse::<IpAddr>()
        .ok()
        .or_else(|| value.parse::<SocketAddr>().ok().map(|address| address.ip()))
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, str::FromStr};

    use axum::{extract::ConnectInfo, http::HeaderMap};

    use super::resolve_client_ip;

    #[test]
    fn forwarded_addresses_are_used_only_when_explicitly_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.8, 198.51.100.20".parse().unwrap(),
        );
        let peer = Some(ConnectInfo(SocketAddr::from_str("10.0.0.4:443").unwrap()));
        assert_eq!(resolve_client_ip(peer, &headers, 0).to_string(), "10.0.0.4");
        assert_eq!(
            resolve_client_ip(peer, &headers, 1).to_string(),
            "198.51.100.20"
        );
    }

    #[test]
    fn caller_supplied_forwarded_address_cannot_spoof_alb_rate_limit_identity() {
        let peer = Some(ConnectInfo(SocketAddr::from_str("10.0.1.20:443").unwrap()));
        let mut first = HeaderMap::new();
        first.insert(
            "x-forwarded-for",
            "198.51.100.10, 203.0.113.8".parse().unwrap(),
        );
        let mut second = HeaderMap::new();
        second.insert(
            "x-forwarded-for",
            "192.0.2.99, 203.0.113.8".parse().unwrap(),
        );

        assert_eq!(
            resolve_client_ip(peer, &first, 1),
            resolve_client_ip(peer, &second, 1)
        );
        assert_eq!(
            resolve_client_ip(peer, &first, 1).to_string(),
            "203.0.113.8"
        );
    }

    #[test]
    fn multiple_trusted_hops_are_counted_from_the_right() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.0.2.99, 203.0.113.8, 10.0.0.4".parse().unwrap(),
        );
        let peer = Some(ConnectInfo(SocketAddr::from_str("10.0.1.20:443").unwrap()));

        assert_eq!(
            resolve_client_ip(peer, &headers, 2).to_string(),
            "203.0.113.8"
        );
    }

    #[test]
    fn invalid_or_incomplete_forwarded_chains_fall_back_to_the_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        let peer = Some(ConnectInfo(SocketAddr::from_str("10.0.1.20:443").unwrap()));

        assert_eq!(
            resolve_client_ip(peer, &headers, 1).to_string(),
            "10.0.1.20"
        );
        assert_eq!(
            resolve_client_ip(peer, &headers, 2).to_string(),
            "10.0.1.20"
        );
    }

    #[test]
    fn forwarded_addresses_may_include_an_alb_preserved_client_port() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.8:49152".parse().unwrap());

        assert_eq!(
            resolve_client_ip(None, &headers, 1).to_string(),
            "203.0.113.8"
        );
    }
}
