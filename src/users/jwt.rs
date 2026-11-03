use anyhow::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub username: String,
    pub first_name: String,
    /// Tokens issued before refresh tokens existed carry no type: they were access tokens.
    #[serde(default = "access_token_type")]
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
}

fn access_token_type() -> String {
    "access".to_string()
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiry_hours: u64,
    refresh_expiry_hours: u64,
    remember_me_hours: u64,
}

impl JwtService {
    pub fn new(secret: &str, expiry_hours: u64, refresh_expiry_hours: u64, remember_me_hours: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiry_hours,
            refresh_expiry_hours,
            remember_me_hours,
        }
    }

    pub fn generate(&self, user_id: i32, username: &str, first_name: &str) -> Result<String> {
        let now = jsonwebtoken::get_current_timestamp();
        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            first_name: first_name.to_string(),
            token_type: "access".to_string(),
            iat: now,
            exp: now + self.expiry_hours * 3600,
        };
        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    pub fn generate_refresh(&self, user_id: i32, username: &str, first_name: &str) -> Result<String> {
        self.mint_refresh(user_id, username, first_name, self.refresh_expiry_hours)
    }

    pub fn generate_remember_me_refresh(&self, user_id: i32, username: &str, first_name: &str) -> Result<String> {
        self.mint_refresh(user_id, username, first_name, self.remember_me_hours)
    }

    fn mint_refresh(&self, user_id: i32, username: &str, first_name: &str, expiry_hours: u64) -> Result<String> {
        let now = jsonwebtoken::get_current_timestamp();
        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            first_name: first_name.to_string(),
            token_type: "refresh".to_string(),
            iat: now,
            exp: now + expiry_hours * 3600,
        };
        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    pub fn verify(&self, token: &str) -> Result<Claims> {
        let data = decode::<Claims>(token, &self.decoding_key, &Validation::default())?;
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_issued_before_token_type_is_an_access_token() {
        #[derive(Serialize)]
        struct LegacyClaims {
            sub: i32,
            username: String,
            first_name: String,
            exp: u64,
            iat: u64,
        }
        let now = jsonwebtoken::get_current_timestamp();
        let legacy = LegacyClaims { sub: 1, username: "alice".into(), first_name: "Alice".into(), exp: now + 3600, iat: now };
        let token = encode(&Header::default(), &legacy, &EncodingKey::from_secret(b"secret")).unwrap();

        let claims = JwtService::new("secret", 1, 1, 1).verify(&token).unwrap();
        assert_eq!(claims.token_type, "access");
    }
}
