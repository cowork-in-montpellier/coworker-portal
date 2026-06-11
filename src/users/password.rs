use base64::{engine::general_purpose::STANDARD, Engine};
use pbkdf2::pbkdf2_hmac;
use rand::{Rng, distributions::Alphanumeric};
use sha2::Sha256;

const DJANGO_ITERATIONS: u32 = 720_000;

pub fn hash_django_password(raw: &str) -> String {
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    let mut dk = [0u8; 32];
    pbkdf2_hmac::<Sha256>(raw.as_bytes(), salt.as_bytes(), DJANGO_ITERATIONS, &mut dk);
    format!("pbkdf2_sha256${}${}${}", DJANGO_ITERATIONS, salt, STANDARD.encode(dk))
}

/// Verifies a plaintext password against a Django-encoded password field.
/// Django format: `pbkdf2_sha256$<iterations>$<salt>$<base64-hash>`
pub fn verify_django_password(raw: &str, encoded: &str) -> bool {
    let parts: Vec<&str> = encoded.splitn(4, '$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }

    let Ok(iterations) = parts[1].parse::<u32>() else {
        return false;
    };
    let salt = parts[2].as_bytes();
    let Ok(stored) = STANDARD.decode(parts[3]) else {
        return false;
    };

    let mut computed = vec![0u8; stored.len()];
    pbkdf2_hmac::<Sha256>(raw.as_bytes(), salt, iterations, &mut computed);

    // Constant-time comparison to prevent timing attacks
    subtle_compare(&computed, &stored)
}

fn subtle_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
