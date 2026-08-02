pub struct PiiGuardrail;

impl PiiGuardrail {
    pub fn new() -> Self {
        Self
    }

    pub fn sanitize(&self, text: &str) -> String {
        let mut result = String::new();
        for word in text.split_whitespace() {
            if word.contains('@') && word.contains('.') {
                result.push_str("[REDACTED_EMAIL] ");
            } else if word.len() >= 10 && word.chars().all(|c| c.is_ascii_digit() || c == '-') {
                result.push_str("[REDACTED_PHONE] ");
            } else {
                result.push_str(word);
                result.push(' ');
            }
        }
        result.trim_end().to_string()
    }
}
