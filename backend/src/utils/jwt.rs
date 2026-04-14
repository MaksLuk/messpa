use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,      // user_id
    pub exp: usize,
    pub iat: usize,
    pub jti: Uuid,
}

pub fn generate_access_token(user_id: Uuid, secret: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp() as usize,
        exp: (now + Duration::minutes(15)).timestamp() as usize,
        jti: Uuid::new_v4(),
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap()
}

pub fn validate_access_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &validation)
        .map(|data| data.claims)
}
