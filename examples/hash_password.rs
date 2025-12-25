use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn main() {
    dotenv::dotenv().ok();

    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "password123".to_string());
    let salt = SaltString::generate(&mut OsRng);

    // Use the same secret as the application
    let secret = std::env::var("SALT").expect("SALT env var required");
    let argon2 = Argon2::new_with_secret(
        secret.as_bytes(),
        argon2::Algorithm::default(),
        argon2::Version::default(),
        argon2::Params::default(),
    )
    .expect("Failed to create Argon2");

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    println!("{}", hash);
}
