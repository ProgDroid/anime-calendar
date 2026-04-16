use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use log::warn;
use secrecy::ExposeSecret as _;

use crate::{config::server::SmtpConfig, error::Error, ServerResult};

#[derive(Clone)]
pub struct EmailService {
    config: SmtpConfig,
}

impl EmailService {
    #[must_use]
    pub const fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// Send a password reset email.
    ///
    /// If `smtp.host` is empty (e.g. local dev), logs the reset URL at WARN
    /// level instead of attempting a connection.
    ///
    /// # Errors
    /// Fails if SMTP connection fails or the email cannot be built/sent.
    pub async fn send_password_reset(&self, to_email: &str, reset_url: &str) -> ServerResult<()> {
        if self.config.host.is_empty() {
            warn!("SMTP not configured — password reset URL for {to_email}: {reset_url}");
            return Ok(());
        }

        let from = self
            .config
            .from_address
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let to = to_email
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject("Reset your Anime Calendar password")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Someone requested a password reset for your Anime Calendar account.\n\n\
                 Click the link below to choose a new password (expires in 1 hour):\n\n\
                 {reset_url}\n\n\
                 If you did not request this, you can safely ignore this email."
            ))
            .map_err(|e| Error::EmailError(e.to_string()))?;

        let creds = Credentials::new(
            self.config.username.clone(),
            self.config.password.expose_secret().to_string(),
        );

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
            .map_err(|e| Error::EmailError(e.to_string()))?
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| Error::EmailError(e.to_string()))?;

        Ok(())
    }

    /// Send an email verification link.
    ///
    /// If `smtp.host` is empty (e.g. local dev), logs the URL at WARN level
    /// instead of attempting a connection.
    ///
    /// # Errors
    /// Fails if SMTP connection fails or the email cannot be built/sent.
    pub async fn send_verification_email(
        &self,
        to_email: &str,
        verify_url: &str,
    ) -> ServerResult<()> {
        if self.config.host.is_empty() {
            warn!(
                "SMTP not configured — email verification URL for {to_email}: {verify_url}"
            );
            return Ok(());
        }

        let from = self
            .config
            .from_address
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let to = to_email
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject("Verify your Anime Calendar email address")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Welcome to Anime Calendar!\n\n\
                 Please verify your email address by clicking the link below \
                 (the link expires in 24 hours):\n\n\
                 {verify_url}\n\n\
                 If you did not create an account, you can safely ignore this email."
            ))
            .map_err(|e| Error::EmailError(e.to_string()))?;

        let creds = Credentials::new(
            self.config.username.clone(),
            self.config.password.expose_secret().to_string(),
        );

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
            .map_err(|e| Error::EmailError(e.to_string()))?
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| Error::EmailError(e.to_string()))?;

        Ok(())
    }
}
