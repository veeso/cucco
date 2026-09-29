//! Emoji shortcode replacement for commit text.

/// Replaces emoji shortcodes such as `:tada:` with the emoji they name.
pub trait ReplaceEmoji {
    /// Returns a copy of the text with every known `:shortcode:` replaced.
    fn replace_emoji_shortcodes(&self) -> String;
}

impl ReplaceEmoji for &str {
    fn replace_emoji_shortcodes(&self) -> String {
        replace_emoji_shortcodes(self.to_string())
    }
}

impl ReplaceEmoji for String {
    fn replace_emoji_shortcodes(&self) -> String {
        replace_emoji_shortcodes(self.to_owned())
    }
}

fn replace_emoji_shortcodes(mut string: String) -> String {
    for emoji in emojis::iter() {
        if let Some(shortcode) = emoji.shortcode() {
            string = string.replace(&format!(":{shortcode}:"), emoji.as_str());
        }
    }

    string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_emoji_shortcodes() {
        let phrase = "yes sir :pinched_fingers: !";

        assert_eq!(replace_emoji_shortcodes(phrase.to_string()), "yes sir 🤌 !");

        assert_eq!(phrase.replace_emoji_shortcodes(), "yes sir 🤌 !");

        assert_eq!(
            phrase.to_string().replace_emoji_shortcodes(),
            "yes sir 🤌 !"
        );
    }
}
