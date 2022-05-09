//! src/domain/subscriber_name.rs

use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        // check if string is actually empty
        let is_empty_or_whitespace = s.trim().is_empty();

        // does string surpass our expected count size?
        let is_too_long = s.graphemes(true).count() > 256;

        // does this contain forbidden characters?
        let forbidden_characters = ['/', '(', ')', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            return Err(format!("{} is not a valid subscriber name.", s));
        }

        Ok(Self(s))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use super::SubscriberName;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "e".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphmemes_is_rejected() {
        let name = "e".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn invalid_characters_are_rejected() {
        let name = "".to_string();
        for name in &['/', '(', ')', '<', '>', '\\', '{', '}'] {
            assert_err!(SubscriberName::parse(name.to_string()));
        }
    }
}
