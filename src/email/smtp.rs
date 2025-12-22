/// SMTP email sending implementation
use super::{EmailConfig, EmailError, EmailResult};
use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

/// Send an email via SMTP
pub async fn send_email(
    config: &EmailConfig,
    to: &str,
    subject: &str,
    body_text: &str,
    body_html: Option<&str>,
) -> EmailResult<()> {
    // Parse email addresses
    let from: Mailbox = format!("{} <{}>", config.from_name, config.from_email)
        .parse()
        .map_err(|e| EmailError::ConfigError(format!("Invalid from address: {}", e)))?;

    let to_string = to.to_string();
    let to: Mailbox = to
        .parse()
        .map_err(|e| EmailError::ConfigError(format!("Invalid to address: {}", e)))?;

    // Build the email
    let email_builder = Message::builder().from(from).to(to).subject(subject);

    // Add body (either plain text only, or multipart with HTML)
    let email = if let Some(html) = body_html {
        email_builder.multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body_text.to_string()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html.to_string()),
                ),
        )?
    } else {
        email_builder
            .header(ContentType::TEXT_PLAIN)
            .body(body_text.to_string())?
    };

    // Create SMTP transport
    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer = if config.use_tls {
        SmtpTransport::relay(&config.smtp_host)?
            .credentials(creds)
            .port(config.smtp_port)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.smtp_host)
            .credentials(creds)
            .port(config.smtp_port)
            .build()
    };

    // Send the email
    mailer.send(&email)?;

    log::info!("Email sent successfully to: {}", to_string);

    Ok(())
}
