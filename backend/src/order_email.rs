//! Production, script-free version of the approved confirmation design.
use std::io::Cursor;

use image::ImageFormat;

use crate::{email::escape_html, orders::Order};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmailLocale {
    Pt,
    #[default]
    En,
    Es,
}

impl EmailLocale {
    pub fn from_code(value: Option<&str>) -> Self {
        match value
            .unwrap_or("en")
            .trim()
            .to_ascii_lowercase()
            .split(['-', '_'])
            .next()
        {
            Some("pt") => Self::Pt,
            Some("es") => Self::Es,
            _ => Self::En,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pt => "pt",
            Self::En => "en",
            Self::Es => "es",
        }
    }
    pub fn text(self, pt: &'static str, en: &'static str, es: &'static str) -> &'static str {
        match self {
            Self::Pt => pt,
            Self::En => en,
            Self::Es => es,
        }
    }
    pub fn money(self, minor: i64, currency: &str) -> String {
        let sign = if minor < 0 { "−" } else { "" };
        let amount = minor.unsigned_abs();
        let decimal = if self == Self::En { '.' } else { ',' };
        let digits = format!("{}{decimal}{:02}", amount / 100, amount % 100);
        if currency == "EUR" {
            if self == Self::En {
                format!("{sign}€{digits}")
            } else {
                format!("{sign}{digits} €")
            }
        } else {
            format!("{sign}{currency} {digits}")
        }
    }
    fn date(self, value: &str) -> String {
        let parts: Vec<_> = value
            .split('T')
            .next()
            .unwrap_or(value)
            .split('-')
            .collect();
        if parts.len() != 3 {
            return value.to_owned();
        }
        let day = parts[2].trim_start_matches('0');
        let month = parts[1].parse::<usize>().unwrap_or(0);
        if !(1..=12).contains(&month) {
            return value.to_owned();
        }
        let months = match self {
            Self::Pt => [
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
            Self::En => [
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
            Self::Es => [
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
        };
        if self == Self::En {
            format!("{day} {} {}", months[month - 1], parts[0])
        } else {
            format!("{day} de {} de {}", months[month - 1], parts[0])
        }
    }
}

#[derive(Clone, Debug)]
pub struct InlineImage {
    pub cid: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
}

pub struct ConfirmationContent {
    pub subject: String,
    pub text: String,
    pub html: String,
    pub images: Vec<InlineImage>,
    pub action_url: String,
}

/// Images are attached with CID references, never public links to private uploads.
pub fn render(
    order: &Order,
    locale: EmailLocale,
    base_url: &str,
    mut images: Vec<InlineImage>,
) -> Result<ConfirmationContent, String> {
    let tr = |pt, en, es| locale.text(pt, en, es);
    let e = escape_html;
    let money = |amount| locale.money(amount, &order.currency);
    let subject = format!(
        "{} — {}",
        tr(
            "A tua encomenda KnitnPrint está confirmada",
            "Your KnitnPrint order is confirmed",
            "Tu pedido de KnitnPrint está confirmado"
        ),
        order.order_number
    );
    let action_url = format!(
        "{}/cart?order_id={}&source=account",
        base_url.trim_end_matches('/'),
        order.id
    );
    let intro = format!(
        "{} {}. {}",
        tr("Olá,", "Hi", "Hola,"),
        order.customer.first_name,
        tr(
            "Recebemos o teu pagamento e vamos preparar cada detalhe com cuidado. A tua ideia está em boas mãos.",
            "We have received your payment and will prepare every detail with care. Your idea is in good hands.",
            "Hemos recibido tu pago y prepararemos cada detalle con cuidado. Tu idea está en buenas manos."
        )
    );
    let title = tr(
        "A tua encomenda está confirmada.",
        "Your order is confirmed.",
        "Tu pedido está confirmado.",
    );
    let ribbon = tr(
        "Feito com cuidado em Portugal",
        "Made with care in Portugal",
        "Hecho con cuidado en Portugal",
    );
    let next = tr(
        "Vamos preparar a tua encomenda com cuidado. As atualizações sobre o transporte e a entrega serão enviadas diretamente pela transportadora.",
        "We will prepare your order with care. Shipping and delivery updates will be sent directly by the carrier.",
        "Prepararemos tu pedido con cuidado. Las actualizaciones sobre el transporte y la entrega las enviará directamente la empresa de transporte.",
    );
    let mut text = format!(
        "{title}\n\n{intro}\n\n{}: {}\n{}: {}\n\n",
        tr("Encomenda", "Order", "Pedido"),
        order.order_number,
        tr("Data", "Date", "Fecha"),
        locale.date(&order.created_at)
    );
    let mut rows = String::new();
    for line in &order.lines {
        let mut previews = String::new();
        let catalog_cid = format!("catalog-{}", line.id);
        if line.customization.is_none() && images.iter().any(|image| image.cid == catalog_cid) {
            previews = format!(
                "<img src=\"cid:{}\" alt=\"{}\" width=\"112\" style=\"display:block;width:112px;max-width:100%;height:auto\">",
                e(&catalog_cid),
                e(&line.product_title)
            );
        }
        if let Some(customization) = &line.customization {
            if let Some(saved) = customization.get("previews").and_then(|v| v.as_array()) {
                for preview in saved {
                    let Some(id) = preview.get("media_id").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let cid = format!("preview-{id}");
                    if images.iter().any(|image| image.cid == cid) {
                        let label = preview
                            .get("view_label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        previews.push_str(&format!("<img src=\"cid:{}\" alt=\"{}\" width=\"112\" style=\"display:block;width:112px;max-width:100%;height:auto;margin-bottom:6px\"><small>{}</small>", e(&cid), e(label), e(label)));
                    }
                }
            }
        }
        let mut details = Vec::new();
        if let Some(areas) = line
            .customization
            .as_ref()
            .and_then(|v| v.get("areas"))
            .and_then(|v| v.as_array())
        {
            for area in areas {
                let view = area
                    .get("view_label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if let Some(content) = area.pointer("/text/content").and_then(|v| v.as_str()) {
                    details.push(format!(
                        "{}: “{content}” · {view}",
                        tr("Texto", "Text", "Texto")
                    ));
                }
                if area.get("photo").is_some() {
                    details.push(format!(
                        "{} · {view}",
                        tr(
                            "Fotografia personalizada",
                            "Personalized photo",
                            "Fotografía personalizada"
                        )
                    ));
                }
            }
        }
        // Never substitute a generic product image for a missing personalized preview.
        if line.customization.is_some() && previews.is_empty() {
            previews = format!(
                "<span style=\"font-size:12px;color:#6e6864\">{}</span>",
                tr(
                    "Personalização guardada na encomenda",
                    "Personalization saved with your order",
                    "Personalización guardada en tu pedido"
                )
            );
        }
        let quantity = tr("Quantidade", "Quantity", "Cantidad");
        let detail_html = details
            .iter()
            .map(|value| {
                format!(
                    "<p style=\"margin:4px 0;color:#5f4652;font-size:12px;line-height:1.6\">{}</p>",
                    e(value)
                )
            })
            .collect::<String>();
        rows.push_str(&format!("<tr><td class=\"product-image\" width=\"124\" valign=\"top\" style=\"padding:18px 12px 18px 0;border-bottom:1px solid #ded4cc\">{previews}</td><td valign=\"top\" style=\"padding:18px 4px;border-bottom:1px solid #ded4cc;overflow-wrap:anywhere\"><strong>{}</strong><p style=\"font-size:12px;color:#6e6864\">{}<br>{quantity}: {} · SKU: {}</p>{detail_html}<p class=\"mobile-price\" style=\"display:none;margin:10px 0 0\">{line_price}</p></td><td class=\"product-price\" align=\"right\" valign=\"top\" style=\"padding:18px 0 18px 6px;border-bottom:1px solid #ded4cc;white-space:nowrap\">{}</td></tr>", e(&line.product_title), e(&line.variant_title), line.quantity, e(&line.sku), e(&money(line.line_total_minor)), line_price=e(&money(line.line_total_minor))));
        text.push_str(&format!(
            "{} — {}\n{quantity}: {} · SKU: {}\n{}\n{}\n\n",
            line.product_title,
            line.variant_title,
            line.quantity,
            line.sku,
            details.join("\n"),
            money(line.line_total_minor)
        ));
    }
    let shipping_label = if order.shipping.method_name == "Shipping not charged" {
        tr(
            "Portes de envio — não cobrados nesta fase",
            "Shipping — not charged at this stage",
            "Envío — sin cargo en esta fase",
        )
    } else {
        tr("Entrega", "Delivery", "Entrega")
    };
    let mut totals = String::new();
    let mut total_row = |label: &str, value: i64, large: bool| {
        totals.push_str(&format!("<tr><td style=\"padding:8px 0;{}\">{}</td><td align=\"right\" style=\"padding:8px 0;white-space:nowrap;{}\">{}</td></tr>", if large { "border-top:1px solid #ded4cc;font:22px Georgia,serif;color:#5f4652" } else { "color:#6e6864" }, e(label), if large { "border-top:1px solid #ded4cc;font-size:22px;color:#5f4652" } else { "" }, e(&money(value))));
        text.push_str(&format!("{label}: {}\n", money(value)));
    };
    total_row(
        tr("Subtotal", "Subtotal", "Subtotal"),
        order.subtotal_minor,
        false,
    );
    if order.discount_minor > 0 {
        let code = order
            .discount
            .as_ref()
            .map_or("", |discount| discount.code.as_str());
        total_row(
            &format!("{} {code}", tr("Desconto", "Discount", "Descuento")),
            -order.discount_minor,
            false,
        );
    }
    total_row(shipping_label, order.shipping_minor, false);
    if order.tax_minor > 0 {
        total_row(tr("Impostos", "Tax", "Impuestos"), order.tax_minor, false);
    }
    total_row(
        tr("Total pago", "Total paid", "Total pagado"),
        order.total_minor,
        true,
    );
    let address = &order.shipping_address;
    let address_lines = [
        &address.recipient_name,
        &address.line1,
        &address.line2,
        &address.postal_code,
        &address.city,
        &address.region,
        &address.country_code,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .cloned()
    .collect::<Vec<_>>();
    let address_html = address_lines
        .iter()
        .map(|s| e(s))
        .collect::<Vec<_>>()
        .join("<br>");
    let delivery_title = tr(
        "Onde vamos entregar",
        "Your delivery address",
        "Dónde entregaremos",
    );
    let payment = tr("Pagamento", "Payment", "Pago");
    let confirmed = tr("Confirmado", "Confirmed", "Confirmado");
    let cta = tr("Ver a minha encomenda", "View my order", "Ver mi pedido");
    let cta_note = tr(
        "Podes acompanhar o estado da encomenda na tua conta.",
        "Follow your order status in your account.",
        "Puedes seguir el estado del pedido en tu cuenta.",
    );
    let next_title = tr("E agora?", "What happens next?", "¿Y ahora?");
    let footer_title = tr(
        "Cada ideia conta uma história.",
        "Every idea tells a story.",
        "Cada idea cuenta una historia.",
    );
    let footer_subtitle = tr(
        "Obrigada por nos deixares fazer parte da tua.",
        "Thank you for letting us be part of yours.",
        "Gracias por dejarnos formar parte de la tuya.",
    );
    let help = tr(
        "Precisas de ajuda? Fala connosco.",
        "Need a hand? We are here to help.",
        "¿Necesitas ayuda? Escríbenos.",
    );
    let eyebrow = tr(
        "Obrigada por escolheres a KnitnPrint",
        "Thank you for choosing KnitnPrint",
        "Gracias por elegir KnitnPrint",
    );
    let items_title = tr(
        "As tuas escolhas",
        "Your chosen pieces",
        "Tus piezas elegidas",
    );
    let html = format!(
        r##"<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>@media(max-width:480px){{.content{{padding:20px!important}}.heading{{font-size:30px!important}}.product-image{{width:80px!important;padding-right:8px!important}}.product-image img{{width:72px!important}}.product-price{{display:none!important}}.mobile-price{{display:block!important}}}}</style></head><body style="margin:0;padding:0;background:#f7f3ef;color:#2e2e2d;font:14px/1.65 Arial,Helvetica,sans-serif"><table role="presentation" width="100%" cellspacing="0" cellpadding="0"><tr><td align="center" style="padding:24px 8px"><table role="presentation" width="620" cellspacing="0" cellpadding="0" style="width:100%;max-width:620px;background:#fffdfc;border:1px solid #ded4cc;table-layout:fixed"><tr><td align="center" style="background:#5f4652;color:#fffdfc;padding:10px 16px;font-size:11px;letter-spacing:1.8px">{ribbon}</td></tr><tr><td align="center" style="background:#f7f3ef;padding:28px 20px"><img src="cid:knitnprint-logo" alt="KnitnPrint" width="280" style="display:block;width:280px;max-width:90%;height:auto"></td></tr><tr><td class="content" align="center" style="background:#f7f3ef;padding:30px 40px;border-top:1px solid #ded4cc"><span style="color:#5f4652;font-size:24px">✓</span><p style="text-transform:uppercase;font-size:11px;letter-spacing:1.4px;color:#5f4652">{eyebrow}</p><h1 class="heading" style="font:35px/1.15 Georgia,serif;color:#5f4652;margin:10px 0">{title}</h1><p style="color:#6e6864;line-height:1.8">{intro}</p></td></tr><tr><td class="content" style="padding:24px 36px"><table role="presentation" width="100%"><tr><td style="font-size:12px;color:#6e6864">{order_label}<br><strong style="color:#2e2e2d">{number}</strong></td><td align="right" style="font-size:12px;color:#6e6864">{date_label}<br><strong style="color:#2e2e2d">{date}</strong></td></tr></table><h2 style="font:21px Georgia,serif;margin:28px 0 0">{items_title}</h2><table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="table-layout:fixed">{rows}</table><table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin-top:16px">{totals}</table><table role="presentation" width="100%" style="margin:26px 0 8px"><tr><td align="center" bgcolor="#5f4652" style="border-radius:5px"><a href="{url}" style="display:block;padding:14px 16px;color:#fffdfc;text-decoration:none">{cta} ↗</a></td></tr></table><p style="text-align:center;font-size:12px;color:#6e6864">{cta_note}</p><table role="presentation" width="100%" bgcolor="#f7f3ef" style="margin-top:24px"><tr><td valign="top" style="padding:20px"><strong>{delivery_title}</strong><p style="font-size:12px;color:#6e6864">{address_html}</p></td><td valign="top" style="padding:20px"><strong>{payment}</strong><p style="color:#5f4652">✓ {confirmed}</p></td></tr></table><h2 style="font:bold 14px Arial,sans-serif;margin-top:26px">{next_title}</h2><p style="font-size:13px;color:#6e6864">{next}</p></td></tr><tr><td align="center" style="background:#f7f3ef;padding:26px 24px"><p style="font:22px Georgia,serif;color:#5f4652;margin:0 0 8px">{footer_title}</p><p style="font-size:12px;color:#6e6864">{footer_subtitle}</p><p style="font-size:12px;color:#6e6864;margin-top:24px">{help}<br><a style="color:#5f4652" href="mailto:support@knitnprint.com">support@knitnprint.com</a></p><p style="font-size:11px;color:#6e6864">KnitnPrint · {ribbon}</p></td></tr></table></td></tr></table></body></html>"##,
        lang = locale.as_str(),
        ribbon = e(ribbon),
        eyebrow = e(eyebrow),
        title = e(title),
        intro = e(&intro),
        order_label = tr("Encomenda", "Order", "Pedido"),
        number = e(&order.order_number),
        date_label = tr("Data", "Date", "Fecha"),
        date = e(&locale.date(&order.created_at)),
        items_title = e(items_title),
        url = e(&action_url)
    );
    text.push_str(&format!("\n{delivery_title}:\n{}\n\n{payment}: {confirmed}\n\n{cta}: {action_url}\n\n{next_title}\n{next}\n\nsupport@knitnprint.com\n", address_lines.join("\n")));
    let mut logo = Cursor::new(Vec::new());
    image::load_from_memory(include_bytes!(
        "../../apps/storefront/public/knitnprint-wordmark.webp"
    ))
    .and_then(|image| image.write_to(&mut logo, ImageFormat::Png))
    .map_err(|_| "could not prepare email logo")?;
    images.push(InlineImage {
        cid: "knitnprint-logo".into(),
        bytes: logo.into_inner(),
        content_type: "image/png".into(),
    });
    Ok(ConfirmationContent {
        subject,
        text,
        html,
        images,
        action_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locales_normalize_and_fall_back_to_english() {
        for code in [None, Some("fr"), Some("")] {
            assert_eq!(EmailLocale::from_code(code), EmailLocale::En);
        }
        assert_eq!(EmailLocale::from_code(Some("PT-pt")), EmailLocale::Pt);
        assert_eq!(EmailLocale::from_code(Some("es_ES")), EmailLocale::Es);
    }
    #[test]
    fn dates_and_money_do_not_use_floating_point() {
        assert_eq!(EmailLocale::Pt.money(2490, "EUR"), "24,90 €");
        assert_eq!(EmailLocale::En.money(2490, "EUR"), "€24.90");
        assert_eq!(EmailLocale::Es.money(-100, "EUR"), "−1,00 €");
        assert_eq!(
            EmailLocale::Pt.date("2026-10-22T00:00:00Z"),
            "22 de outubro de 2026"
        );
        assert_eq!(
            EmailLocale::En.date("2026-10-22T00:00:00Z"),
            "22 October 2026"
        );
        assert_eq!(
            EmailLocale::Es.date("2026-10-22T00:00:00Z"),
            "22 de octubre de 2026"
        );
    }
}
