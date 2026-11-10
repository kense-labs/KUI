//! Compact currency formatting for monetary readouts.

pub(crate) fn format_money(value: f64, currency: &str, locale: &str) -> String {
    let digits = match currency {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    };
    if !value.is_finite() {
        return "—".into();
    }
    let lang = locale.split(['-', '_']).next().unwrap_or("");
    let (group, decimal, suffix) = match lang {
        "fr" => ('\u{202f}', ',', true),
        "de" | "es" | "it" | "pt" => ('.', ',', true),
        _ => (',', '.', false),
    };
    let symbol = match (currency, locale) {
        ("USD", "en-US") => "$",
        ("USD", _) => "US$",
        ("EUR", _) => "€",
        ("GBP", _) => "£",
        ("JPY", _) => "¥",
        ("CNY", _) => "CN¥",
        ("CAD", "en-CA") | ("AUD", "en-AU") => "$",
        ("CAD", _) => "CA$",
        ("AUD", _) => "A$",
        (code, _) => code,
    };
    let plain = format!("{:.*}", digits, value.abs());
    let (whole, fraction) = plain.split_once('.').unwrap_or((&plain, ""));
    let mut grouped = String::with_capacity(plain.len() + 8);
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(group);
        }
        grouped.push(digit);
    }
    if !fraction.is_empty() {
        grouped.push(decimal);
        grouped.push_str(fraction);
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if suffix {
        format!("{sign}{grouped}\u{a0}{symbol}")
    } else if symbol.len() == 3 && symbol.bytes().all(|b| b.is_ascii_uppercase()) {
        format!("{sign}{symbol}\u{a0}{grouped}")
    } else {
        format!("{sign}{symbol}{grouped}")
    }
}
