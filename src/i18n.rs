include!("i18n_table.rs");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Ru,
    En,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == "en" {
            Language::En
        } else {
            Language::Ru
        }
    }
}

pub struct Vars<'a>(pub &'a [(&'a str, &'a str)]);

impl<'a> Vars<'a> {
    fn get(&self, name: &str) -> Option<&str> {
        self.0.iter().find(|(key, _)| *key == name).map(|(_, value)| *value)
    }
}

pub fn text(lang: Language, key: &str) -> String {
    raw(lang, key).unwrap_or(key).to_string()
}

pub fn format(lang: Language, key: &str, vars: &[(&str, &str)]) -> String {
    substitute(raw(lang, key).unwrap_or(key), &Vars(vars))
}

fn raw(lang: Language, key: &str) -> Option<&'static str> {
    lookup(lang, key).or_else(|| lookup(Language::Ru, key))
}

fn substitute(template: &str, vars: &Vars) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find('}') {
            let name = &rest[start + 1..start + end];
            out.push_str(vars.get(name).unwrap_or(""));
            rest = &rest[start + end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

pub fn error_text(lang: Language, code: &str) -> String {
    let key = format!("error.{code}");
    if lookup(lang, &key).is_some() || lookup(Language::Ru, &key).is_some() {
        text(lang, &key)
    } else {
        text(lang, "error.INTERNAL")
    }
}

pub fn format_bytes(lang: Language, bytes: u64) -> String {
    let units: [&str; 4] = match lang {
        Language::Ru => ["Б", "КБ", "МБ", "ГБ"],
        Language::En => ["B", "KB", "MB", "GB"],
    };
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let digits = if unit == 0 || value >= 100.0 { 0 } else { 1 };
    let mut rendered = format!("{value:.*}", digits);
    if lang == Language::Ru {
        rendered = rendered.replace('.', ",");
    }
    format!("{rendered} {}", units[unit])
}

pub fn format_duration(lang: Language, seconds: f64) -> String {
    let s = seconds.round().max(1.0) as u64;
    if s < 60 {
        return match lang {
            Language::Ru => format!("{s} с"),
            Language::En => format!("{s}s"),
        };
    }
    let m = ((s as f64) / 60.0).round() as u64;
    match lang {
        Language::Ru => format!("{m} мин"),
        Language::En => format!("{m} min"),
    }
}

pub fn format_month_date(lang: Language, year: i32, month: u32, day: u32) -> String {
    let names: [&str; 12] = match lang {
        Language::Ru => [
            "января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября",
            "октября", "ноября", "декабря",
        ],
        Language::En => [
            "January", "February", "March", "April", "May", "June", "July", "August", "September",
            "October", "November", "December",
        ],
    };
    let name = names[(month.clamp(1, 12) - 1) as usize];
    match lang {
        Language::Ru => format!("{day} {name} {year} г."),
        Language::En => format!("{day} {name} {year}"),
    }
}
