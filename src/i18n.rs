include!("i18n_table.rs");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Ru,
    En,
}

impl Language {
    pub fn code(self) -> String {
        match self {
            Language::Ru => crate::secret!("ru").to_owned(),
            Language::En => crate::secret!("en").to_owned(),
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == crate::secret!("en") {
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
    let key = format!("{}{code}", crate::secret!("error."));
    if lookup(lang, &key).is_some() || lookup(Language::Ru, &key).is_some() {
        text(lang, &key)
    } else {
        text(lang, crate::secret!("error.INTERNAL"))
    }
}

pub fn format_bytes(lang: Language, bytes: u64) -> String {
    let units: [String; 4] = match lang {
        Language::Ru => [crate::secret!("Б").to_owned(), crate::secret!("КБ").to_owned(), crate::secret!("МБ").to_owned(), crate::secret!("ГБ").to_owned()],
        Language::En => [crate::secret!("B").to_owned(), crate::secret!("KB").to_owned(), crate::secret!("MB").to_owned(), crate::secret!("GB").to_owned()],
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
        rendered = rendered.replace('.', crate::secret!(","));
    }
    format!("{rendered} {}", units[unit])
}

pub fn format_duration(lang: Language, seconds: f64) -> String {
    let s = seconds.round().max(1.0) as u64;
    if s < 60 {
        return match lang {
            Language::Ru => format!("{s} {}", crate::secret!("с")),
            Language::En => format!("{s}{}", crate::secret!("s")),
        };
    }
    let m = ((s as f64) / 60.0).round() as u64;
    match lang {
        Language::Ru => format!("{m} {}", crate::secret!("мин")),
        Language::En => format!("{m} {}", crate::secret!("min")),
    }
}

pub fn format_month_date(lang: Language, year: i32, month: u32, day: u32) -> String {
    let names: [String; 12] = match lang {
        Language::Ru => [
            crate::secret!("января").to_owned(), crate::secret!("февраля").to_owned(), crate::secret!("марта").to_owned(), crate::secret!("апреля").to_owned(), crate::secret!("мая").to_owned(), crate::secret!("июня").to_owned(), crate::secret!("июля").to_owned(), crate::secret!("августа").to_owned(), crate::secret!("сентября").to_owned(),
            crate::secret!("октября").to_owned(), crate::secret!("ноября").to_owned(), crate::secret!("декабря").to_owned(),
        ],
        Language::En => [
            crate::secret!("January").to_owned(), crate::secret!("February").to_owned(), crate::secret!("March").to_owned(), crate::secret!("April").to_owned(), crate::secret!("May").to_owned(), crate::secret!("June").to_owned(), crate::secret!("July").to_owned(), crate::secret!("August").to_owned(), crate::secret!("September").to_owned(),
            crate::secret!("October").to_owned(), crate::secret!("November").to_owned(), crate::secret!("December").to_owned(),
        ],
    };
    let name = &names[(month.clamp(1, 12) - 1) as usize];
    match lang {
        Language::Ru => format!("{day} {name} {year} {}", crate::secret!("г.")),
        Language::En => format!("{day} {name} {year}"),
    }
}
