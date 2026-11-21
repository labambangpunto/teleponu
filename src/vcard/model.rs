pub const MAX_FULL_NAME_LEN: usize = 30;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructuredName {
    pub family: String,
    pub given: String,
    pub middle: String,
    pub prefix: String,
    pub suffix: String,
}

impl StructuredName {
    pub fn to_full_name(&self) -> String {
        [
            self.prefix.trim(),
            self.given.trim(),
            self.middle.trim(),
            self.family.trim(),
            self.suffix.trim(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypedValue {
    pub types: Vec<String>,
    pub value: String,
}

impl TypedValue {
    pub fn types_string(&self) -> String {
        self.types.join(",")
    }

    pub fn set_types_from_str(&mut self, raw: &str) {
        self.types = raw
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    pub types: Vec<String>,
    pub po_box: String,
    pub extended: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
}

impl Address {
    pub fn types_string(&self) -> String {
        self.types.join(",")
    }

    pub fn set_types_from_str(&mut self, raw: &str) {
        self.types = raw
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
    }

    pub fn summary(&self) -> String {
        let parts: Vec<&str> = [
            self.street.trim(),
            self.city.trim(),
            self.region.trim(),
            self.postal_code.trim(),
            self.country.trim(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect();

        if parts.is_empty() {
            "Empty address".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VCard {
    pub n: StructuredName,
    pub fn_name: String,
    pub org: String,
    pub title: String,
    pub phones: Vec<TypedValue>,
    pub emails: Vec<TypedValue>,
    pub urls: Vec<String>,
    pub addresses: Vec<Address>,
    pub note: String,
    /// Preserves unhandled vCard 3.0 properties (e.g., BDAY, PHOTO, X-*) when saving.
    pub extra_raw_lines: Vec<String>,
}

impl VCard {
    pub fn new_blank() -> Self {
        Self {
            n: StructuredName {
                given: "New".to_string(),
                family: "Contact".to_string(),
                ..Default::default()
            },
            fn_name: "New Contact".to_string(),
            phones: vec![TypedValue {
                types: vec!["CELL".to_string(), "VOICE".to_string()],
                value: String::new(),
            }],
            ..Default::default()
        }
    }

    pub fn display_name(&self) -> String {
        let trimmed = self.fn_name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        let from_n = self.n.to_full_name();
        if !from_n.is_empty() {
            return from_n;
        }
        "(No Name)".to_string()
    }

    pub fn primary_subtitle(&self) -> String {
        if let Some(phone) = self.phones.iter().find(|p| !p.value.trim().is_empty()) {
            return phone.value.trim().to_string();
        }
        if let Some(email) = self.emails.iter().find(|e| !e.value.trim().is_empty()) {
            return email.value.trim().to_string();
        }
        if !self.org.trim().is_empty() {
            return self.org.trim().to_string();
        }
        "No phone number".to_string()
    }

    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        let effective_name = {
            let from_n = self.n.to_full_name();
            if from_n.is_empty() {
                self.fn_name.trim().to_string()
            } else {
                from_n
            }
        };

        if let Some(err) = validate_full_name(&effective_name) {
            errors.push(err.to_string());
        }

        for phone in &self.phones {
            if let Some(err) = validate_phone(&phone.value) {
                errors.push(format!("Phone '{}': {}", phone.value.trim(), err));
            }
        }

        for email in &self.emails {
            if let Some(err) = validate_email(&email.value) {
                errors.push(format!("Email '{}': {}", email.value.trim(), err));
            }
        }

        for url in &self.urls {
            if let Some(err) = validate_url(url) {
                errors.push(format!("Website '{}': {}", url.trim(), err));
            }
        }

        for addr in &self.addresses {
            if let Some(err) = validate_postal_code(&addr.postal_code) {
                errors.push(format!(
                    "Postal code '{}': {}",
                    addr.postal_code.trim(),
                    err
                ));
            }
        }

        errors
    }
}

pub fn validate_full_name(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Some("Name is required in vCard 3.0.")
    } else if trimmed.chars().count() > MAX_FULL_NAME_LEN {
        Some("Full name must not exceed 30 characters including spaces.")
    } else {
        None
    }
}

pub fn validate_phone(raw: &str) -> Option<&'static str> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if s.starts_with('0') {
        return Some("Country code is required (e.g., +62), cannot start with 0.");
    }

    let Some(rest) = s.strip_prefix('+') else {
        return Some("Must start with '+' and a country code (e.g., +628123456789).");
    };

    if rest.starts_with('0') {
        return Some("Invalid country code after '+' (cannot start with 0).");
    }

    if !rest
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '-' | '(' | ')'))
    {
        return Some("May only contain digits, spaces, parentheses, or hyphens.");
    }

    let digit_count = rest.chars().filter(|c| c.is_ascii_digit()).count();
    if digit_count < 7 {
        return Some("Too short (minimum 7 digits including country code).");
    }
    if digit_count > 15 {
        return Some("Too long (maximum 15 digits).");
    }

    None
}

pub fn validate_email(raw: &str) -> Option<&'static str> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if s.chars().any(|c| c.is_whitespace()) {
        return Some("Email address cannot contain spaces.");
    }

    let at_count = s.matches('@').count();
    if at_count == 0 {
        return Some("Must contain '@' (e.g., name@example.com).");
    }
    if at_count > 1 {
        return Some("Cannot contain more than one '@' symbol.");
    }

    let (local, domain) = s.split_once('@').unwrap();
    if local.is_empty() {
        return Some("Missing username before '@'.");
    }
    if domain.is_empty() {
        return Some("Missing domain name after '@'.");
    }
    if !domain.contains('.') {
        return Some("Domain must include an extension (e.g., .com, .co.id).");
    }
    if domain.starts_with('.') || domain.ends_with('.') || domain.contains("..") {
        return Some("Invalid domain format after '@'.");
    }

    let tld = domain.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some("Domain extension must be at least 2 letters (e.g., .com).");
    }

    None
}

pub fn validate_url(raw: &str) -> Option<&'static str> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if s.chars().any(|c| c.is_whitespace()) {
        return Some("Website URL cannot contain spaces.");
    }

    let lower = s.to_ascii_lowercase();
    let without_scheme = if let Some(rest) = lower.strip_prefix("https://") {
        rest
    } else if let Some(rest) = lower.strip_prefix("http://") {
        rest
    } else {
        return Some("Must start with https:// or http:// (e.g., https://example.com).");
    };

    let host = without_scheme
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or("");

    if host.is_empty()
        || !host.contains('.')
        || host.starts_with('.')
        || host.ends_with('.')
        || host.contains("..")
    {
        return Some("Must include a valid domain and extension (e.g., https://example.com).");
    }

    let tld = host.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some("Website domain extension must be at least 2 letters.");
    }

    None
}

pub fn validate_postal_code(raw: &str) -> Option<&'static str> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if !(3..=10).contains(&s.len())
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-'))
    {
        return Some("Postal code must be 3–10 letters or digits.");
    }

    None
}

