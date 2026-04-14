use lettre::{Message, SmtpTransport, Transport, message::header::ContentType};

pub async fn send_mail(
    smtp_host: &str,
    smtp_port: u16,
    from: &str,
    to: &str,
    header: &str,
    message: String,
) -> Result<(), String> {
    let email = Message::builder()
        .from(from.parse().map_err(|_| "Ошибка при парсинге email отправителя".to_string())?)
        .to(to.parse().map_err(|_| "Ошибка при парсинге email получателя".to_string())?)
        .subject(header)
        .header(ContentType::TEXT_HTML)
        .body(message)
        .map_err(|e| format!("Ошибка построения сообщения: {e}"))?;

    let mailer = SmtpTransport::builder_dangerous(smtp_host)
        .port(smtp_port)
        .build();

    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Ошибка отправки сообщения: {e}")),
    }
}
