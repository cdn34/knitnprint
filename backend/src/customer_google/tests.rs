use super::*;
use serde_json::{Value, json};
async fn body(response: Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn validates_google_signature_audience_issuer_expiry_and_verified_email() {
    let service = GoogleService {
        client_id: Some("test.apps.googleusercontent.com".into()),
        ..Default::default()
    };
    *service.keys.lock().await = Some(CachedKeys {
        expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3600),
        refreshed_at: std::time::Instant::now(),
        keys: serde_json::from_str(include_str!("../../tests/fixtures/google-test-jwks.json"))
            .unwrap(),
    });
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(include_bytes!(
        "../../tests/fixtures/google-test-key.pem"
    ))
    .unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("test".into());
    let claims = json!({"sub":"google-subject","email":"marta@gmail.com","email_verified":true,"nonce":"nonce","aud":"test.apps.googleusercontent.com","iss":"https://accounts.google.com","exp":jsonwebtoken::get_current_timestamp()+600});
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    assert!(service.verify(&token).await.is_ok());
    for (field, value) in [
        ("aud", json!("attacker")),
        ("iss", json!("https://attacker.test")),
        ("exp", json!(1)),
        ("email_verified", json!(false)),
        ("sub", json!("")),
    ] {
        let mut invalid = claims.clone();
        invalid[field] = value;
        assert!(
            service
                .verify(&jsonwebtoken::encode(&header, &invalid, &key).unwrap())
                .await
                .is_err(),
            "accepted invalid {field}"
        );
    }
    let mut parts: Vec<_> = token.split('.').map(str::to_owned).collect();
    parts[2].replace_range(..1, "!");
    assert!(service.verify(&parts.join(".")).await.is_err());
    header.kid = Some("unknown-key".into());
    let unknown = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            service.verify(&unknown)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(service.verify("not-a-jwt").await.is_err());
}
async fn prepared(
    pool: &PgPool,
    email: &str,
    subject: &str,
    trusted: bool,
    customer: Option<Uuid>,
) -> (Uuid, CookieJar) {
    let id = Uuid::now_v7();
    let browser = new_session_token();
    sqlx::query("INSERT INTO customer_google_challenges(id,browser_hash,nonce_hash,purpose,subject,email,first_name,last_name,trusted_email,customer_id) VALUES($1,$2,$3,'sign_in',$4,$5,'Marta','Silva',$6,$7)").bind(id).bind(hash_token(&browser).as_slice()).bind(hash_token("nonce").as_slice()).bind(subject).bind(email).bind(trusted).bind(customer).execute(pool).await.unwrap();
    (id, CookieJar::new().add(Cookie::new(BROWSER, browser)))
}
fn confirmation(id: Uuid) -> GoogleConfirmRequest {
    GoogleConfirmRequest {
        challenge_id: id,
        password: None,
        token: None,
        first_name: None,
        last_name: None,
    }
}
#[tokio::test]
async fn account_linking_methods_recovery_and_erasure() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("google_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |conn, _| {
            let query = format!("SET search_path TO {search},public");
            Box::pin(async move {
                sqlx::query(&query).execute(conn).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let google = GoogleService {
        client_id: Some("test.apps.googleusercontent.com".into()),
        ..Default::default()
    };
    *google.keys.lock().await = Some(CachedKeys {
        expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3600),
        refreshed_at: std::time::Instant::now(),
        keys: serde_json::from_str(include_str!("../../tests/fixtures/google-test-jwks.json"))
            .unwrap(),
    });
    let state = AppState {
        google,
        database: Some(pool.clone()),
        email: crate::email::EmailService::development("http://localhost:3000"),
        ..Default::default()
    };
    // Exercise the public credential exchange, including nonce binding and replay rejection.
    let response = challenge(
        State(state.clone()),
        ClientIp("127.0.0.1".parse().unwrap()),
        CookieJar::new(),
        Json(ChallengeRequest {
            purpose: "sign_in".into(),
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[axum::http::header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_owned();
    let browser_jar = CookieJar::new().add(Cookie::new(BROWSER, cookie));
    let challenge_body = body(response).await;
    let challenge_id = Uuid::parse_str(challenge_body["id"].as_str().unwrap()).unwrap();
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(include_bytes!(
        "../../tests/fixtures/google-test-key.pem"
    ))
    .unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("test".into());
    let mut claims = json!({"sub":"api-subject","email":"api@gmail.com","email_verified":true,"nonce":"wrong-nonce","given_name":"API","family_name":"Customer","aud":"test.apps.googleusercontent.com","iss":"https://accounts.google.com","exp":jsonwebtoken::get_current_timestamp()+600});
    let wrong = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let response = authenticate(
        State(state.clone()),
        ClientIp("127.0.0.1".parse().unwrap()),
        browser_jar.clone(),
        Json(GoogleRequest {
            challenge_id,
            credential: wrong,
        }),
    )
    .await;
    assert_eq!(
        body(response).await["error"]["code"],
        "invalid_google_nonce"
    );
    claims["nonce"] = challenge_body["nonce"].clone();
    let credential = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let response = authenticate(
        State(state.clone()),
        ClientIp("127.0.0.1".parse().unwrap()),
        browser_jar.clone(),
        Json(GoogleRequest {
            challenge_id,
            credential: credential.clone(),
        }),
    )
    .await;
    let session_cookie = response
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .find(|h| h.starts_with(account::CUSTOMER_SESSION_COOKIE))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let session_jar = CookieJar::new().add(Cookie::parse(session_cookie).unwrap().into_owned());
    let authenticated = body(response).await;
    assert_eq!(authenticated["status"], "authenticated");
    assert!(
        authenticated["google_authenticated_until"]
            .as_i64()
            .unwrap()
            > jsonwebtoken::get_current_timestamp() as i64
    );
    let api_identity = AuthenticatedCustomer {
        id: Uuid::parse_str(authenticated["profile"]["id"].as_str().unwrap()).unwrap(),
        email: "api@gmail.com".into(),
        first_name: "API".into(),
        last_name: "Customer".into(),
        phone: "".into(),
        email_verified: true,
    };
    let refreshed = body(
        account::me(
            State(state.clone()),
            session_jar.clone(),
            api_identity.clone(),
        )
        .await,
    )
    .await;
    assert_eq!(
        refreshed["google_authenticated_until"],
        authenticated["google_authenticated_until"]
    );
    // Another session for the same account must not inherit the Google proof.
    let other_token = new_session_token();
    let mut tx = pool.begin().await.unwrap();
    account::insert_session(
        &mut tx,
        Uuid::now_v7(),
        api_identity.id,
        &hash_token(&other_token),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let other_jar = CookieJar::new().add(Cookie::new(
        account::CUSTOMER_SESSION_COOKIE,
        other_token.clone(),
    ));
    assert!(
        body(
            account::me(
                State(state.clone()),
                other_jar.clone(),
                api_identity.clone()
            )
            .await
        )
        .await["google_authenticated_until"]
            .is_null()
    );
    sqlx::query("UPDATE customer_sessions SET reauthenticated_at=now()-interval '6 minutes',reauthenticated_method='google' WHERE token_hash=$1").bind(hash_token(&other_token).as_slice()).execute(&pool).await.unwrap();
    assert!(body(account::me(State(state.clone()),other_jar,api_identity.clone()).await).await["google_authenticated_until"].is_null());
    assert_eq!(
        password(
            State(state.clone()),
            session_jar,
            api_identity,
            Json(PasswordRequest {
                password: "fresh-google-password".into()
            })
        )
        .await
        .status(),
        StatusCode::OK
    );
    // Deferred completion must not turn a six-minute-old Google proof into fresh authentication.
    let (old_challenge, old_jar) =
        prepared(&pool, "delayed@gmail.com", "delayed-subject", true, None).await;
    sqlx::query(
        "UPDATE customer_google_challenges SET created_at=now()-interval '6 minutes' WHERE id=$1",
    )
    .bind(old_challenge)
    .execute(&pool)
    .await
    .unwrap();
    let delayed = body(complete(state.clone(), old_jar, confirmation(old_challenge)).await).await;
    assert_eq!(delayed["status"], "authenticated");
    assert!(delayed["google_authenticated_until"].is_null());

    assert_ne!(
        authenticate(
            State(state.clone()),
            ClientIp("127.0.0.1".parse().unwrap()),
            browser_jar,
            Json(GoogleRequest {
                challenge_id,
                credential
            })
        )
        .await
        .status(),
        StatusCode::OK
    );
    let (challenge, jar) = prepared(&pool, "marta@gmail.com", "subject-one", true, None).await;
    let response = complete(state.clone(), jar.clone(), confirmation(challenge)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = body(response).await;
    let id = Uuid::parse_str(data["profile"]["id"].as_str().unwrap()).unwrap();
    assert_eq!(data["profile"]["auth_methods"]["password"], false);
    assert_ne!(
        complete(state.clone(), jar, confirmation(challenge))
            .await
            .status(),
        StatusCode::OK,
        "challenge replay"
    );
    // Returning sign-in preserves the local ID; mismatched Google email never moves accounts.
    let (challenge, jar) = prepared(&pool, "marta@gmail.com", "subject-one", true, Some(id)).await;
    assert_eq!(
        body(complete(state.clone(), jar, confirmation(challenge)).await).await["profile"]["id"],
        id.to_string()
    );
    let (challenge, jar) =
        prepared(&pool, "changed@gmail.com", "subject-one", true, Some(id)).await;
    assert_ne!(
        complete(state.clone(), jar, confirmation(challenge))
            .await
            .status(),
        StatusCode::OK
    );
    // A password created before mailbox verification must not survive trusted ownership proof.
    let victim = Uuid::now_v7();
    sqlx::query("INSERT INTO customers(id,customer_type,email,first_name,last_name) VALUES($1,'registered','victim@gmail.com','Original','Name')").bind(victim).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO customer_accounts(customer_id,password_hash) VALUES($1,$2)")
        .bind(victim)
        .bind(hash_password("attacker-passphrase").unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let (challenge, jar) = prepared(
        &pool,
        "victim@gmail.com",
        "subject-victim",
        true,
        Some(victim),
    )
    .await;
    let result = body(complete(state.clone(), jar, confirmation(challenge)).await).await;
    assert_eq!(result["profile"]["id"], victim.to_string());
    assert_eq!(result["profile"]["auth_methods"]["password"], false);
    // External addresses require fresh mailbox proof, bound to this browser.
    let (challenge, jar) = prepared(&pool, "external@example.test", "external", false, None).await;
    assert_eq!(
        body(complete(state.clone(), jar.clone(), confirmation(challenge)).await).await["status"],
        "confirmation_required"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM customers WHERE email='external@example.test'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let token = new_session_token();
    sqlx::query("UPDATE customer_google_challenges SET confirmation_hash=$2 WHERE id=$1")
        .bind(challenge)
        .bind(hash_token(&token).as_slice())
        .execute(&pool)
        .await
        .unwrap();
    let mut input = confirmation(challenge);
    input.token = Some(token.clone());
    assert_ne!(
        complete(state.clone(), CookieJar::new(), input)
            .await
            .status(),
        StatusCode::OK
    );
    let mut input = confirmation(challenge);
    input.token = Some(token);
    assert_eq!(
        body(complete(state.clone(), jar, input).await).await["status"],
        "authenticated"
    );
    // Fresh Google proof can add a password, but a stale/revoked session cannot modify methods.
    let session = new_session_token();
    let mut tx = pool.begin().await.unwrap();
    account::insert_session(&mut tx, Uuid::now_v7(), id, &hash_token(&session))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE customer_sessions SET reauthenticated_at=now(),reauthenticated_method='google' WHERE token_hash=$1").bind(hash_token(&session).as_slice()).execute(&pool).await.unwrap();
    let identity = || AuthenticatedCustomer {
        id,
        email: "marta@gmail.com".into(),
        first_name: "Marta".into(),
        last_name: "Silva".into(),
        phone: "".into(),
        email_verified: true,
    };
    let session_jar = CookieJar::new().add(Cookie::new(account::CUSTOMER_SESSION_COOKIE, session));
    let response = mutate(
        state.clone(),
        session_jar.clone(),
        identity(),
        Some("new-password-passphrase".into()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(methods(&pool, id).await.unwrap().password);
    assert_ne!(
        mutate(
            state.clone(),
            session_jar,
            identity(),
            None,
            Some("google".into())
        )
        .await
        .status(),
        StatusCode::OK
    );
    // A Google connection locks both email-change request and confirmation paths.
    let response = account::request_email_change(
        State(state.clone()),
        CookieJar::new(),
        ClientIp("127.0.0.1".parse().unwrap()),
        identity(),
        Json(account::RequestEmailChangeRequest {
            email: "different@example.test".into(),
        }),
    )
    .await;
    assert_eq!(body(response).await["error"]["code"], "google_email_locked");
    let email_token = new_session_token();
    sqlx::query("INSERT INTO customer_account_tokens(id,customer_id,token_kind,token_hash,target_email,expires_at) VALUES($1,$2,'email_change',$3,'different@example.test',now()+interval '1 hour')").bind(Uuid::now_v7()).bind(id).bind(hash_token(&email_token).as_slice()).execute(&pool).await.unwrap();
    assert_eq!(
        account::confirm_email_change(
            State(state.clone()),
            Json(account::AccountTokenRequest { token: email_token })
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    // Concurrent removals cannot leave the customer without a usable method.
    async fn proof(pool: &PgPool, id: Uuid, method: &str) -> CookieJar {
        let token = new_session_token();
        let mut tx = pool.begin().await.unwrap();
        account::insert_session(&mut tx, Uuid::now_v7(), id, &hash_token(&token))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        sqlx::query("UPDATE customer_sessions SET reauthenticated_at=now(),reauthenticated_method=$2 WHERE token_hash=$1").bind(hash_token(&token).as_slice()).bind(method).execute(pool).await.unwrap();
        CookieJar::new().add(Cookie::new(account::CUSTOMER_SESSION_COOKIE, token))
    }
    let password_proof = proof(&pool, id, "password").await;
    let google_proof = proof(&pool, id, "google").await;
    let (first, second) = tokio::join!(
        mutate(
            state.clone(),
            password_proof,
            identity(),
            None,
            Some("google".into())
        ),
        mutate(
            state.clone(),
            google_proof,
            identity(),
            None,
            Some("password".into())
        )
    );
    assert_eq!(
        [first.status(), second.status()]
            .iter()
            .filter(|s| **s == StatusCode::OK)
            .count(),
        1
    );
    let remaining = methods(&pool, id).await.unwrap();
    assert_ne!(remaining.password, remaining.google_email.is_some());
    // Explicit unlinking requires confirmation even for a trusted matching address.
    sqlx::query("UPDATE customer_accounts SET password_hash=$2 WHERE customer_id=$1")
        .bind(victim)
        .bind(hash_password("owner-password-passphrase").unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let victim_identity = || AuthenticatedCustomer {
        id: victim,
        email: "victim@gmail.com".into(),
        first_name: "Original".into(),
        last_name: "Name".into(),
        phone: "".into(),
        email_verified: true,
    };
    let jar = proof(&pool, victim, "password").await;
    assert_eq!(
        mutate(
            state.clone(),
            jar,
            victim_identity(),
            None,
            Some("google".into())
        )
        .await
        .status(),
        StatusCode::OK
    );
    let (challenge, jar) = prepared(
        &pool,
        "victim@gmail.com",
        "subject-victim",
        true,
        Some(victim),
    )
    .await;
    assert_eq!(
        body(complete(state.clone(), jar.clone(), confirmation(challenge)).await).await["status"],
        "confirmation_required"
    );
    let mut input = confirmation(challenge);
    input.password = Some("owner-password-passphrase".into());
    assert_eq!(
        body(complete(state.clone(), jar, input).await).await["status"],
        "authenticated"
    );
    // Restore Google on the first test account if concurrent removal disconnected it.
    if methods(&pool, id).await.unwrap().google_email.is_none() {
        sqlx::query("INSERT INTO customer_google_identities(customer_id,subject,email) VALUES($1,'subject-one','marta@gmail.com')").bind(id).execute(&pool).await.unwrap();
    }
    // Password recovery also works when Google is the only login method.
    sqlx::query("UPDATE customer_accounts SET password_hash=NULL WHERE customer_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let token = new_session_token();
    sqlx::query("INSERT INTO customer_account_tokens(id,customer_id,token_kind,token_hash,expires_at) VALUES($1,$2,'password_reset',$3,now()+interval '1 hour')").bind(Uuid::now_v7()).bind(id).bind(hash_token(&token).as_slice()).execute(&pool).await.unwrap();
    assert_eq!(
        account::reset_password(
            State(state.clone()),
            CookieJar::new(),
            Json(account::ResetPasswordRequest {
                token,
                password: "recovered-passphrase".into()
            })
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let m = methods(&pool, id).await.unwrap();
    assert!(m.password);
    assert!(m.google_email.is_some());
    let (pending, _) = prepared(&pool, "marta@gmail.com", "subject-one", true, Some(id)).await;
    sqlx::query("DELETE FROM customer_accounts WHERE customer_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM customer_google_identities WHERE customer_id=$1"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM customer_google_challenges WHERE id=$1")
            .bind(pending)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
