use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use log::warn;
use secrecy::ExposeSecret as _;

use crate::{ServerResult, config::server::SmtpConfig, error::Error};

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
            warn!("SMTP not configured — email verification URL for {to_email}: {verify_url}");
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


    /// Send a co-editor invitation link.
    ///
    /// Plain-text body matches the existing password-reset / verify-email
    /// convention — calendar names and owner usernames are user-supplied,
    /// so plain text avoids HTML injection without an escape dependency.
    /// If `smtp.host` is empty (e.g. local dev), logs the invite URL at WARN
    /// level instead of attempting a connection.
    ///
    /// # Errors
    /// Fails if SMTP connection fails or the email cannot be built/sent.
    pub async fn send_invitation(
        &self,
        to_email: &str,
        owner_display: &str,
        calendar_name: &str,
        invite_url: &str,
    ) -> ServerResult<()> {
        if self.config.host.is_empty() {
            warn!(
                "SMTP not configured — invitation URL for {to_email} to \"{calendar_name}\": {invite_url}"
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
            .subject(format!("Edit access invite: {calendar_name}"))
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "{owner_display} invited you to edit \"{calendar_name}\" on Anime Calendar.\n\n\
                 Accept the invitation by clicking the link below \
                 (the link expires in 7 days):\n\n\
                 {invite_url}\n\n\
                 If you don't recognize this invitation, you can safely ignore this email."
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

    /// Notify a previously-suspended editor that their access has been
    /// restored (owner re-upgraded to Pro). Used by the suspend/restore flow
    /// in Phase 4. Plain-text body, same logging fallback as the other
    /// senders.
    ///
    /// # Errors
    /// Fails if SMTP connection fails or the email cannot be built/sent.
    pub async fn send_editor_restored(
        &self,
        to_email: &str,
        calendar_name: &str,
    ) -> ServerResult<()> {
        if self.config.host.is_empty() {
            warn!(
                "SMTP not configured — editor restored notice for {to_email} on \"{calendar_name}\""
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
            .subject(format!("Your editor access on \"{calendar_name}\" is back"))
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Good news — your editor access on \"{calendar_name}\" has been restored.\n\n\
                 You can now add and remove items as before. No action is needed on your end."
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
