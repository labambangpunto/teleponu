use super::model::VCard;

fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

pub fn serialize_vcf(cards: &[VCard]) -> String {
    let mut lines: Vec<String> = Vec::new();

    for card in cards {
        lines.push("BEGIN:VCARD".to_string());
        lines.push("VERSION:3.0".to_string());

        lines.push(format!(
            "N:{};{};{};{};{}",
            escape_text(&card.n.family),
            escape_text(&card.n.given),
            escape_text(&card.n.middle),
            escape_text(&card.n.prefix),
            escape_text(&card.n.suffix),
        ));

        let fn_value = if card.fn_name.trim().is_empty() {
            card.display_name()
        } else {
            card.fn_name.clone()
        };
        lines.push(format!("FN:{}", escape_text(&fn_value)));

        if !card.org.trim().is_empty() {
            lines.push(format!("ORG:{}", escape_text(&card.org)));
        }

        if !card.title.trim().is_empty() {
            lines.push(format!("TITLE:{}", escape_text(&card.title)));
        }

        for phone in &card.phones {
            if phone.value.trim().is_empty() {
                continue;
            }
            if phone.types.is_empty() {
                lines.push(format!("TEL:{}", escape_text(&phone.value)));
            } else {
                lines.push(format!(
                    "TEL;TYPE={}:{}",
                    phone.types.join(","),
                    escape_text(&phone.value)
                ));
            }
        }

        for email in &card.emails {
            if email.value.trim().is_empty() {
                continue;
            }
            if email.types.is_empty() {
                lines.push(format!("EMAIL:{}", escape_text(&email.value)));
            } else {
                lines.push(format!(
                    "EMAIL;TYPE={}:{}",
                    email.types.join(","),
                    escape_text(&email.value)
                ));
            }
        }

        for url in &card.urls {
            if !url.trim().is_empty() {
                lines.push(format!("URL:{}", escape_text(url)));
            }
        }

        for addr in &card.addresses {
            let has_content = [
                &addr.po_box,
                &addr.extended,
                &addr.street,
                &addr.city,
                &addr.region,
                &addr.postal_code,
                &addr.country,
            ]
            .iter()
            .any(|f| !f.trim().is_empty());

            if !has_content {
                continue;
            }

            let formatted_addr = format!(
                "{};{};{};{};{};{};{}",
                escape_text(&addr.po_box),
                escape_text(&addr.extended),
                escape_text(&addr.street),
                escape_text(&addr.city),
                escape_text(&addr.region),
                escape_text(&addr.postal_code),
                escape_text(&addr.country),
            );

            if addr.types.is_empty() {
                lines.push(format!("ADR:{}", formatted_addr));
            } else {
                lines.push(format!(
                    "ADR;TYPE={}:{}",
                    addr.types.join(","),
                    formatted_addr
                ));
            }
        }

        if !card.note.trim().is_empty() {
            lines.push(format!("NOTE:{}", escape_text(&card.note)));
        }

        for extra in &card.extra_raw_lines {
            if !extra.trim().is_empty() {
                lines.push(extra.clone());
            }
        }

        lines.push("END:VCARD".to_string());
    }

    let mut output = lines.join("\r\n");
    if !output.is_empty() {
        output.push_str("\r\n");
    }
    output
}
