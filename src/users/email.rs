use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
}

pub(crate) async fn send_smtp_email(
    smtp: &SmtpConfig,
    to_email: &str,
    subject: &str,
    body: String,
) -> anyhow::Result<()> {
    let email = Message::builder()
        .from(smtp.from_email.parse()?)
        .to(to_email.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    let creds = Credentials::new(smtp.username.clone(), smtp.password.clone());
    let mailer = if smtp.port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)?
    }
    .port(smtp.port)
    .credentials(creds)
    .build();

    mailer.send(email).await?;
    Ok(())
}
