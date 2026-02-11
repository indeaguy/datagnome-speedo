use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParametersBuilder};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_address: String,
    /// When true, accept TLS certs with hostname mismatch (e.g. SMTP_HOST is IP or different name).
    pub tls_skip_verify: bool,
}

pub async fn send_newsletter(
    config: &EmailConfig,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let email = Message::builder()
        .from(config.from_address.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .to(to.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(
        config.smtp_user.clone(),
        config.smtp_pass.clone(),
    );

    let mailer: AsyncSmtpTransport<Tokio1Executor> = if config.tls_skip_verify {
        let tls_params = TlsParametersBuilder::new(config.smtp_host.clone())
            .dangerous_accept_invalid_hostnames(true)
            .build_native()
            .map_err(|e| e.to_string())?;
        match config.smtp_port {
            465 => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .map_err(|e| e.to_string())?
                .port(config.smtp_port)
                .credentials(creds)
                .tls(Tls::Wrapper(tls_params))
                .build(),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                .map_err(|e| e.to_string())?
                .port(config.smtp_port)
                .credentials(creds)
                .tls(Tls::Required(tls_params))
                .build(),
        }
    } else {
        match config.smtp_port {
            465 => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .map_err(|e| e.to_string())?
                .port(config.smtp_port)
                .credentials(creds)
                .build(),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                .map_err(|e| e.to_string())?
                .port(config.smtp_port)
                .credentials(creds)
                .build(),
        }
    };

    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}
