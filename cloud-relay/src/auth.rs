use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const JWT_SECRET: &[u8] = b"pc-butler-secret-key-change-in-production";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub exp: usize,
}

pub fn generate_token(user_id: &str, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = chrono_timestamp() + 86400 * 30; // 30 days
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(data.claims)
}

fn chrono_timestamp() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
}

pub fn generate_pairing_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1_000_000))
}

pub fn generate_device_token() -> String {
    use rand::Rng;
    let rng = rand::rng();
    let bytes: Vec<u8> = rng.sample_iter(rand::distr::StandardUniform).take(32).collect();
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
