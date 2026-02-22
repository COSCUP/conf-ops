use mail_parser::{MessageParser, MimeHeaders};

use super::error::EmailError;

#[derive(Debug, Clone)]
pub struct ParsedEmail {
    pub from_name: Option<String>,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Vec<String>,
    pub subject: String,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub attachments: Vec<ParsedAttachment>,
    pub raw_headers: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ParsedAttachment {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// Parse raw MIME bytes into a structured `ParsedEmail`.
///
/// # Errors
///
/// Returns `EmailError::MimeParseError` if the MIME data is invalid.
pub fn parse_mime(raw: &[u8]) -> Result<ParsedEmail, EmailError> {
    let message = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| EmailError::MimeParseError("Failed to parse MIME message".to_string()))?;

    let (from_name, from_address) = message.from().and_then(|addrs| addrs.first()).map_or_else(
        || (None, String::new()),
        |addr| {
            (
                addr.name().map(String::from),
                addr.address().unwrap_or_default().to_string(),
            )
        },
    );

    if from_address.is_empty() {
        return Err(EmailError::MimeParseError(
            "Missing From address".to_string(),
        ));
    }

    let to_addresses = message
        .to()
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|a| a.address().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    let cc_addresses = message
        .cc()
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|a| a.address().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    let subject = message.subject().unwrap_or_default().to_string();

    let message_id = message.message_id().map(|s| format!("<{s}>"));

    let in_reply_to = message
        .in_reply_to()
        .as_text_list()
        .and_then(|list| list.first().map(|s| format!("<{s}>")));

    let references: Vec<String> = message
        .references()
        .as_text_list()
        .map(|list| list.iter().map(|s| format!("<{s}>")).collect())
        .unwrap_or_default();

    let html_body = message.body_html(0).map(|s| s.to_string());
    let text_body = message.body_text(0).map(|s| s.to_string());

    let mut attachments = Vec::new();
    for part in message.attachments() {
        let filename = part.attachment_name().unwrap_or("unnamed").to_string();
        let mime_type = part.content_type().map_or_else(
            || "application/octet-stream".to_string(),
            |ct: &mail_parser::ContentType| {
                let main = ct.ctype();
                ct.subtype()
                    .map_or_else(|| main.to_string(), |sub| format!("{main}/{sub}"))
            },
        );
        let data = part.contents().to_vec();
        attachments.push(ParsedAttachment {
            filename,
            mime_type,
            data,
        });
    }

    let mut headers_map = serde_json::Map::new();
    for header in message.headers() {
        let name = header.name().to_string();
        let value = header.value().as_text().unwrap_or_default().to_string();
        headers_map.insert(name, serde_json::Value::String(value));
    }
    let raw_headers = serde_json::Value::Object(headers_map);

    Ok(ParsedEmail {
        from_name,
        from_address,
        to_addresses,
        cc_addresses,
        subject,
        message_id,
        in_reply_to,
        references,
        html_body,
        text_body,
        attachments,
        raw_headers,
    })
}

/// Normalize an email subject by removing `Re:`/`Fwd:`/`Fw:` prefixes.
pub fn normalize_subject(subject: &str) -> String {
    let mut s = subject.trim();
    loop {
        let lower = s.to_lowercase();
        if let Some(rest) = lower
            .strip_prefix("re:")
            .or_else(|| lower.strip_prefix("fwd:"))
            .or_else(|| lower.strip_prefix("fw:"))
        {
            s = &s[s.len() - rest.len()..];
            s = s.trim_start();
        } else {
            break;
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_re_prefix() {
        assert_eq!(normalize_subject("Re: Hello World"), "Hello World");
        assert_eq!(normalize_subject("RE: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Re: Re: Hello"), "Hello");
    }

    #[test]
    fn normalize_removes_fwd_prefix() {
        assert_eq!(normalize_subject("Fwd: Hello World"), "Hello World");
        assert_eq!(normalize_subject("FWD: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Fw: Hello World"), "Hello World");
    }

    #[test]
    fn normalize_removes_mixed_prefixes() {
        assert_eq!(normalize_subject("Re: Fwd: Hello"), "Hello");
        assert_eq!(normalize_subject("Fwd: Re: Hello"), "Hello");
    }

    #[test]
    fn normalize_preserves_clean_subject() {
        assert_eq!(normalize_subject("Hello World"), "Hello World");
        assert_eq!(normalize_subject("  Hello World  "), "Hello World");
    }

    #[test]
    fn parse_simple_mime() {
        let raw = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Test Subject\r\n\
                    Message-ID: <test-123@example.com>\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    Hello, world!";

        let parsed = parse_mime(raw).expect("should parse");
        assert_eq!(parsed.from_address, "sender@example.com");
        assert_eq!(parsed.to_addresses, vec!["recipient@example.com"]);
        assert_eq!(parsed.subject, "Test Subject");
        assert_eq!(
            parsed.message_id,
            Some("<test-123@example.com>".to_string())
        );
        assert_eq!(parsed.text_body, Some("Hello, world!".to_string()));
    }

    #[test]
    fn parse_mime_with_reply_headers() {
        let raw = b"From: reply@example.com\r\n\
                    To: original@example.com\r\n\
                    Subject: Re: Original Subject\r\n\
                    Message-ID: <reply-456@example.com>\r\n\
                    In-Reply-To: <original-123@example.com>\r\n\
                    References: <original-123@example.com>\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    This is a reply.";

        let parsed = parse_mime(raw).expect("should parse");
        assert_eq!(
            parsed.in_reply_to,
            Some("<original-123@example.com>".to_string())
        );
        assert!(!parsed.references.is_empty());
    }
}
