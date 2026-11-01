//! Production, table-based version of the approved confirmation design.
//! Personalization images are immutable order snapshots, never catalog substitutes.
use crate::{email::escape_html, orders::Order};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct InlineImage {
    pub cid: String,
    pub bytes: Vec<u8>,
}

pub struct RenderedConfirmation {
    pub subject: String,
    pub text: String,
    pub html: String,
}

pub fn preview_cid(media_id: Uuid) -> String {
    format!("preview-{media_id}@knitnprint")
}

pub fn render(order: &Order, locale: &str, base_url: &str) -> RenderedConfirmation {
    let locale = if matches!(locale, "pt" | "es") {
        locale
    } else {
        "en"
    };
    let translations: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_str(include_str!("../assets/order-email-copy.json"))
            .expect("bundled translations");
    let copy = &translations[locale];
    let t = |key: &str| copy.get(key).map(String::as_str).unwrap_or("");
    let h = escape_html;
    let money = |amount| format_money(amount, &order.currency, locale);
    let url = format!("{base_url}/cart?order_id={}&source=account", order.id);
    let date = localized_date(&order.created_at, locale);
    let mut text = format!(
        "{}\n\n{} {}. {}\n\n{}: {}\n{}: {}\n\n{}\n",
        t("title"),
        t("greeting"),
        order.customer.first_name,
        t("introBody"),
        t("orderLabel"),
        order.order_number,
        t("dateLabel"),
        date,
        t("itemsTitle")
    );
    let mut items = String::new();
    for line in &order.lines {
        let options = line
            .variant_options
            .as_object()
            .map(|values| {
                values
                    .iter()
                    .map(|(key, value)| {
                        format!(
                            "{key}: {}",
                            value
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| value.to_string())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .unwrap_or_default();
        let mut details = vec![line.variant_title.clone()];
        if !options.is_empty() {
            details.push(options);
        }
        details.push(format!("{} {}", t("quantity"), line.quantity));
        let details = details.join(" · ");
        let mut personalization = Vec::new();
        if let Some(customization) = &line.customization {
            if let Some(areas) = customization.get("areas").and_then(Value::as_array) {
                for area in areas {
                    personalization.push(describe_area(area, t("text"), t("photo")));
                }
            } else {
                personalization.push(describe_area(customization, t("text"), t("photo")));
            }
        }
        let previews: Vec<crate::carts::PersonalizationPreview> =
            serde_json::from_value(line.personalization_previews.clone()).unwrap_or_default();
        let mut visuals = String::new();
        for preview in previews {
            visuals.push_str(&format!("<p style=\"margin:0 0 8px\"><img src=\"cid:{}\" width=\"112\" alt=\"{}\" style=\"display:block;width:112px;max-width:100%;height:auto;border:0\"><span style=\"font-size:11px;color:#6e6864\">{}</span></p>", preview_cid(preview.media_id), h(&preview.view_label), h(&preview.view_label)));
        }
        if visuals.is_empty() && line.customization.is_some() {
            visuals = format!(
                "<span style=\"font-size:11px;color:#6e6864\">{}</span>",
                h(t("missingPreview"))
            );
        }
        let personal = personalization
            .iter()
            .filter(|value| !value.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        text.push_str(&format!(
            "\n{}\n{}\n{}\n{}: {} · {}\n",
            line.product_title,
            details,
            personal,
            t("unit"),
            money(line.unit_price_minor),
            money(line.line_total_minor)
        ));
        items.push_str(&format!(r#"<tr><td width="120" valign="top" style="padding:18px 8px 18px 0;border-bottom:1px solid #ded4cc">{visuals}</td><td valign="top" style="padding:18px 8px;border-bottom:1px solid #ded4cc"><h3 style="margin:0 0 5px;font-size:16px;color:#5f4652">{}</h3><p style="margin:0 0 6px;font-size:12px;color:#6e6864">{}</p><p style="margin:0;font-size:12px;color:#8a6877">{}</p><p style="margin:6px 0 0;font-size:11px;color:#6e6864">{}: {}</p></td><td valign="top" align="right" style="padding:18px 0;border-bottom:1px solid #ded4cc;white-space:nowrap;font-weight:bold;color:#5f4652">{}</td></tr>"#, h(&line.product_title), h(&details), h(&personal), h(t("unit")), h(&money(line.unit_price_minor)), h(&money(line.line_total_minor))));
    }
    let mut totals = String::new();
    for (label, value) in [
        (t("subtotal").to_owned(), order.subtotal_minor),
        (
            format!(
                "{} {}",
                t("discount"),
                order
                    .discount
                    .as_ref()
                    .map(|discount| discount.code.as_str())
                    .unwrap_or("")
            ),
            -order.discount_minor,
        ),
        (t("shipping").to_owned(), order.shipping_minor),
        (t("tax").to_owned(), order.tax_minor),
        (t("total").to_owned(), order.total_minor),
    ] {
        if value == 0 && label.starts_with(t("discount")) {
            continue;
        }
        let grand = label == t("total");
        totals.push_str(&format!("<tr><td style=\"padding:8px 0;{}\">{}</td><td align=\"right\" style=\"padding:8px 0;{}\">{}</td></tr>", if grand { "border-top:2px solid #5f4652;font-weight:bold;font-size:19px;color:#5f4652" } else { "color:#6e6864" }, h(&label), if grand { "border-top:2px solid #5f4652;font-weight:bold;font-size:19px;color:#5f4652" } else { "color:#5f4652" }, h(&money(value))));
        text.push_str(&format!("{label}: {}\n", money(value)));
    }
    let address = &order.shipping_address;
    let address_lines = [
        &address.recipient_name,
        &address.line1,
        &address.line2,
        &format!("{} {}", address.postal_code, address.city),
        &address.region,
        &address.country_code,
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .cloned()
    .collect::<Vec<_>>();
    text.push_str(&format!(
        "\n{}\n{}\n\n{}: {}\n\n{}\n{}\n\n{}: {}\nsupport@knitnprint.com\n",
        t("deliveryTitle"),
        address_lines.join("\n"),
        t("paymentTitle"),
        t("paymentConfirmed"),
        t("nextTitle"),
        t("nextBody"),
        t("cta"),
        url
    ));
    let address_html = address_lines
        .iter()
        .map(|line| h(line))
        .collect::<Vec<_>>()
        .join("<br>");
    let html = format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{subject}</title>
</head>
<body style="margin:0;padding:0;background:#f7f3ef;color:#2e2e2d;font:14px/1.65 Arial,Helvetica,sans-serif">
<div style="display:none;max-height:0;overflow:hidden">{title} {order_number}</div>
<table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0" style="background:#f7f3ef">
<tr>
<td align="center" style="padding:24px 8px">
<table role="presentation" width="620" cellspacing="0" cellpadding="0" border="0" style="width:100%;max-width:620px;background:#fffdfc;border:1px solid #ded4cc">
<tr>
<td align="center" style="padding:10px 18px;background:#5f4652;color:#fffdfc;font-size:11px;letter-spacing:2px">{ribbon}</td>
</tr>
<tr>
<td align="center" style="padding:28px 24px;background:#f7f3ef">
<img src="cid:wordmark@knitnprint" width="280" alt="KnitnPrint" style="display:block;width:280px;max-width:90%;height:auto;border:0">
</td>
</tr>
<tr>
<td align="center" style="padding:30px 24px;background:#f7f3ef;border-top:1px solid #ded4cc">
<span style="color:#5f4652;font-size:32px">&#10003;</span>
<p style="font-size:10px;letter-spacing:2px;color:#8a6877">{eyebrow}</p>
<h1 style="margin:12px 0;color:#5f4652;font:32px/1.15 Georgia,serif">{title}</h1>
<p style="color:#6e6864">{greeting} {name}. {intro}</p>
</td>
</tr>
<tr>
<td style="padding:24px">
<table role="presentation" width="100%">
<tr>
<td style="color:#6e6864;font-size:11px">{order_label}<br>
<strong style="color:#5f4652;font-size:13px">{order_number}</strong>
</td>
<td align="right" style="color:#6e6864;font-size:11px">{date_label}<br>
<strong style="color:#5f4652;font-size:13px">{date}</strong>
</td>
</tr>
</table>
<h2 style="margin:28px 0 0;font:24px Georgia,serif;color:#5f4652">{items_title}</h2>
<table role="presentation" width="100%" cellspacing="0" cellpadding="0">{items}</table>
<table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="margin-top:18px">{totals}</table>
<table role="presentation" width="100%">
<tr>
<td align="center" style="padding:30px 0">
<a href="{url}" style="display:inline-block;padding:14px 28px;background:#5f4652;color:#fffdfc;text-decoration:none;font-weight:bold;border-radius:4px">{cta}</a>
<p style="font-size:11px;color:#6e6864">{cta_note}</p>
</td>
</tr>
</table>
<table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background:#f7f3ef">
<tr>
<td valign="top" style="padding:20px 12px">
<h2 style="font-size:14px;color:#5f4652">{delivery_title}</h2>
<p style="font-size:12px;color:#6e6864">{address_html}</p>
</td>
<td valign="top" style="padding:20px 12px">
<h2 style="font-size:14px;color:#5f4652">{payment_title}</h2>
<p style="font-size:12px;color:#5f4652">&#10003; {confirmed}</p>
</td>
</tr>
</table>
<h2 style="margin-top:28px;font-size:16px;color:#5f4652">{next_title}</h2>
<p style="color:#6e6864;font-size:12px">{next_body}</p>
</td>
</tr>
<tr>
<td align="center" style="padding:30px 24px;border-top:1px solid #ded4cc;background:#f7f3ef">
<p style="font:24px Georgia,serif;color:#5f4652">{footer_title}</p>
<p style="color:#6e6864;font-size:12px">{footer_subtitle}</p>
<p style="margin-top:24px;font-size:12px">{help}<br>
<a href="mailto:support@knitnprint.com" style="color:#5f4652">support@knitnprint.com</a>
</p>
<p style="font-size:10px;color:#6e6864">KnitnPrint · {ribbon}</p>
</td>
</tr>
</table>
</td>
</tr>
</table>
</body></html>"#,
        subject = h(t("subject")),
        title = h(t("title")),
        order_number = h(&order.order_number),
        ribbon = h(t("ribbon")),
        eyebrow = h(t("eyebrow")),
        greeting = h(t("greeting")),
        name = h(&order.customer.first_name),
        intro = h(t("introBody")),
        order_label = h(t("orderLabel")),
        date_label = h(t("dateLabel")),
        date = h(&date),
        items_title = h(t("itemsTitle")),
        url = h(&url),
        cta = h(t("cta")),
        cta_note = h(t("ctaNote")),
        delivery_title = h(t("deliveryTitle")),
        payment_title = h(t("paymentTitle")),
        confirmed = h(t("paymentConfirmed")),
        next_title = h(t("nextTitle")),
        next_body = h(t("nextBody")),
        footer_title = h(t("footerTitle")),
        footer_subtitle = h(t("footerSubtitle")),
        help = h(t("help"))
    );
    RenderedConfirmation {
        subject: format!("{} · {}", t("subject"), order.order_number),
        text,
        html,
    }
}

fn describe_area(area: &Value, text_label: &str, photo_label: &str) -> String {
    let mut details = Vec::new();
    for key in ["view_label", "area_label"] {
        if let Some(value) = area.get(key).and_then(Value::as_str) {
            details.push(value.to_owned());
        }
    }
    if let Some(value) = area.pointer("/text/content").and_then(Value::as_str) {
        details.push(format!("{text_label}: “{value}”"));
    }
    if area.get("photo").is_some_and(|value| !value.is_null()) {
        details.push(photo_label.to_owned());
    }
    details.join(" · ")
}

pub fn format_money(amount: i64, currency: &str, locale: &str) -> String {
    let absolute = amount.unsigned_abs();
    let decimal = if locale == "en" { '.' } else { ',' };
    let number = format!(
        "{}{}{decimal}{:02}",
        if amount < 0 { "−" } else { "" },
        absolute / 100,
        absolute % 100
    );
    if currency == "EUR" {
        if locale == "en" {
            format!("€{number}")
        } else {
            format!("{number} €")
        }
    } else {
        format!("{currency} {number}")
    }
}

fn localized_date(value: &str, locale: &str) -> String {
    let date = value.get(..10).unwrap_or(value);
    let parts: Vec<_> = date.split('-').collect();
    if parts.len() != 3 {
        return date.to_owned();
    }
    let month = parts[1]
        .parse::<usize>()
        .ok()
        .filter(|month| (1..=12).contains(month));
    let Some(month) = month else {
        return date.to_owned();
    };
    let names = match locale {
        "pt" => [
            "janeiro",
            "fevereiro",
            "março",
            "abril",
            "maio",
            "junho",
            "julho",
            "agosto",
            "setembro",
            "outubro",
            "novembro",
            "dezembro",
        ],
        "es" => [
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ],
        _ => [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ],
    };
    let day = parts[2].trim_start_matches('0');
    if locale == "en" {
        format!("{} {} {}", day, names[month - 1], parts[0])
    } else {
        format!("{} de {} de {}", day, names[month - 1], parts[0])
    }
}

pub async fn load_images(
    pool: &sqlx::PgPool,
    storage: Option<&crate::object_storage::ObjectStorage>,
    order: &Order,
) -> Result<Vec<InlineImage>, String> {
    let logo = png(include_bytes!("../assets/wordmark.webp"))?;
    let mut images = vec![InlineImage {
        cid: "wordmark@knitnprint".into(),
        bytes: logo,
    }];
    let mut seen = std::collections::BTreeSet::new();
    for line in &order.lines {
        let previews: Vec<crate::carts::PersonalizationPreview> =
            serde_json::from_value(line.personalization_previews.clone())
                .map_err(|_| "order preview snapshot is invalid")?;
        for preview in previews {
            if !seen.insert(preview.media_id) {
                continue;
            }
            let storage = storage.ok_or("object storage is required for saved order previews")?;
            let key = sqlx::query_scalar::<_, String>("SELECT variant.object_key FROM media_variants variant JOIN media_assets media ON media.id=variant.media_asset_id JOIN order_line_personalization_media retained ON retained.media_asset_id=media.id WHERE retained.order_line_id=$1 AND media.id=$2 AND media.status='ready' AND media.scan_status='clean' AND variant.kind='card'")
                .bind(line.id).bind(preview.media_id).fetch_optional(pool).await.map_err(|_| "saved order preview could not be loaded")?.ok_or("saved order preview is unavailable")?;
            let object = storage
                .get(&key)
                .await
                .map_err(|_| "saved order preview could not be read")?;
            let bytes = tokio::task::spawn_blocking(move || png(&object.bytes))
                .await
                .map_err(|_| "saved order preview could not be decoded")??;
            images.push(InlineImage {
                cid: preview_cid(preview.media_id),
                bytes,
            });
            if images.iter().map(|image| image.bytes.len()).sum::<usize>() > 15_000_000 {
                return Err("order preview attachments exceed the email size limit".into());
            }
        }
    }
    Ok(images)
}

fn png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|_| "email image could not be decoded")?
        .thumbnail(600, 600);
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "email image could not be encoded")?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn localized_money_is_exact_and_handles_negative_discounts() {
        assert_eq!(format_money(4392, "EUR", "pt"), "43,92 €");
        assert_eq!(format_money(-438, "EUR", "en"), "€−4.38");
        assert_eq!(
            format_money(i64::MIN, "USD", "es"),
            "USD −92233720368547758,08"
        );
    }
    #[test]
    fn dates_use_the_checkout_language() {
        assert_eq!(
            localized_date("2026-10-20T12:00:00Z", "pt"),
            "20 de outubro de 2026"
        );
        assert_eq!(
            localized_date("2026-10-20T12:00:00Z", "en"),
            "20 October 2026"
        );
        assert_eq!(
            localized_date("2026-10-20T12:00:00Z", "es"),
            "20 de octubre de 2026"
        );
    }
    #[test]
    fn logo_is_a_mail_compatible_png_and_translations_have_identical_keys() {
        assert!(
            png(include_bytes!("../assets/wordmark.webp"))
                .unwrap()
                .starts_with(b"\x89PNG")
        );
        let copies: BTreeMap<String, BTreeMap<String, String>> =
            serde_json::from_str(include_str!("../assets/order-email-copy.json")).unwrap();
        assert!(copies["pt"]["introBody"].contains("cada detalhe com cuidado"));
        assert!(copies["en"]["introBody"].contains("every detail with care"));
        assert!(copies["es"]["introBody"].contains("cada detalle con cuidado"));
        for locale in ["pt", "es"] {
            assert_eq!(
                copies[locale].keys().collect::<Vec<_>>(),
                copies["en"].keys().collect::<Vec<_>>()
            );
        }
    }
}
