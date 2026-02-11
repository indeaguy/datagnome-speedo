use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use rocket::request::Outcome;
use rocket::request::{FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Uuid,
    pub email: Option<String>,
}

pub struct User(pub UserContext);

/// Supabase can send aud as a string or array; accept both so decoding does not fail.
fn deserialize_aud<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Aud {
        S(String),
        A(Vec<String>),
    }
    let v = Option::<Aud>::deserialize(d)?;
    Ok(match v {
        None => None,
        Some(Aud::S(s)) => Some(s),
        Some(Aud::A(a)) => a.into_iter().next(),
    })
}

#[derive(Debug, Deserialize)]
struct SupabaseJwtClaims {
    sub: String,
    exp: i64,
    email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_aud")]
    aud: Option<String>,
}

pub enum JwtConfig {
    LegacySecret {
        secret: Arc<[u8]>,
        audience: Option<String>,
    },
    Jwks {
        jwks_url: String,
        issuer: String,
        audience: Option<String>,
    },
}

/// Derives Supabase project URL from a Supabase Postgres DATABASE_URL
/// (e.g. postgresql://...@db.PROJECT_REF.supabase.co:5432/postgres -> https://PROJECT_REF.supabase.co).
fn supabase_url_from_database_url(database_url: &str) -> Option<String> {
    let after_at = database_url.find("@db.")?;
    let start = after_at + 4;
    let rest = &database_url[start..];
    let end = rest.find(".supabase.co")?;
    let project_ref = &rest[..end];
    if project_ref.is_empty() {
        return None;
    }
    Some(format!("https://{}.supabase.co", project_ref))
}

impl JwtConfig {
    /// Build from env: use SUPABASE_JWT_SECRET (legacy) if set, else SUPABASE_URL or DATABASE_URL for JWKS.
    pub fn from_env(
        jwt_secret: Option<&str>,
        audience: Option<String>,
        database_url: Option<&str>,
        supabase_url: Option<&str>,
    ) -> Result<Self, String> {
        if let Some(secret) = jwt_secret {
            if !secret.is_empty() {
                return Ok(JwtConfig::LegacySecret {
                    secret: Arc::from(secret.as_bytes()),
                    audience,
                });
            }
        }
        let url = supabase_url
            .map(String::from)
            .filter(|s| !s.is_empty())
            .or_else(|| database_url.and_then(|u| supabase_url_from_database_url(u)));
        if let Some(url) = url {
            return Ok(JwtConfig::Jwks {
                jwks_url: format!("{}/auth/v1/.well-known/jwks.json", url.trim_end_matches('/')),
                issuer: format!("{}/auth/v1", url.trim_end_matches('/')),
                audience,
            });
        }
        Err("Set SUPABASE_JWT_SECRET (legacy), or SUPABASE_URL, or a Supabase DATABASE_URL for JWT signing keys".into())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let config = match req.rocket().state::<JwtConfig>() {
            Some(c) => c,
            None => return Outcome::Error((rocket::http::Status::InternalServerError, ())),
        };
        let auth_header = req.headers().get_one("Authorization");
        let token = match auth_header {
            Some(h) if h.starts_with("Bearer ") => h.trim_start_matches("Bearer ").trim(),
            _ => {
                eprintln!("[auth] 401: missing or invalid Authorization header (expected Bearer <token>)");
                return Outcome::Error((rocket::http::Status::Unauthorized, ()));
            }
        };

        let token_data = match config {
            JwtConfig::LegacySecret { secret, audience } => {
                let mut validation = Validation::default();
                validation.validate_exp = true;
                if let Some(ref aud) = audience {
                    validation.set_audience(&[aud]);
                }
                match decode::<SupabaseJwtClaims>(
                    token,
                    &DecodingKey::from_secret(secret),
                    &validation,
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("[auth] 401: legacy JWT decode failed: {}", e);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                }
            }
            JwtConfig::Jwks {
                jwks_url,
                issuer,
                audience,
            } => {
                let header = match decode_header(token) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("[auth] 401: JWT header decode failed: {}", e);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                };
                let kid = match header.kid {
                    Some(k) => k,
                    None => {
                        eprintln!("[auth] 401: JWT has no kid (key id)");
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                };
                let jwks: JwkSet = match reqwest::get(jwks_url.as_str()).await {
                    Ok(r) => match r.json().await {
                        Ok(j) => j,
                        Err(e) => {
                            eprintln!("[auth] 401: JWKS fetch parse error: {}", e);
                            return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                        }
                    },
                    Err(e) => {
                        eprintln!("[auth] 401: JWKS fetch failed (check SUPABASE_URL and network): {}", e);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                };
                let jwk = match jwks.find(&kid) {
                    Some(k) => k,
                    None => {
                        eprintln!("[auth] 401: JWKS has no key for kid={:?}", kid);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                };
                let decoding_key = match DecodingKey::from_jwk(jwk) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("[auth] 401: DecodingKey from_jwk failed: {}", e);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                };
                // Use the algorithm from the token header (Supabase uses ES256 with JWKS).
                let mut validation = Validation::new(header.alg);
                validation.validate_exp = true;
                validation.set_issuer(&[issuer]);
                if let Some(ref aud) = audience {
                    validation.set_audience(&[aud]);
                }
                match decode::<SupabaseJwtClaims>(token, &decoding_key, &validation) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("[auth] 401: JWT decode/validation failed (issuer/aud/exp?): {}", e);
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }
                }
            }
        };

        let user_id = match Uuid::parse_str(&token_data.claims.sub) {
            Ok(u) => u,
            Err(_) => {
                eprintln!("[auth] 401: JWT sub is not a valid UUID");
                return Outcome::Error((rocket::http::Status::Unauthorized, ()));
            }
        };
        Outcome::Success(User(UserContext {
            user_id,
            email: token_data.claims.email,
        }))
    }
}
