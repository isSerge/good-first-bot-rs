use crate::bot_handler::CallbackAction;

/// Converts a GitHub color hex code to an emoji representation.
pub fn github_color_to_emoji(hex_color: &str) -> &str {
    match hex_color.to_lowercase().as_str() {
        // Reds / Pinks
        "b60205" | "d73a4a" | "e99695" | "f9d0c4" | "ffc0cb" | "d0312d" => "🔴",

        // Oranges
        "f29513" | "f8c99c" | "fb6a06" | "d93f0b" | "ff8c00" | "ffaf1c" => "🟠",

        // Yellows / Golds
        "fef2c0" | "fbca04" | "e4e669" | "ffeb3b" | "f9e076" | "fadc73" => "🟡",

        // Greens
        "0e8a16" | "006b75" | "5ab302" | "a2eeef" | "008672" | "c2e0c6" | "1aa34a" | "4caf50" => {
            "🟢"
        }

        // Blues / Teals
        "0052cc" | "c5def5" | "0075ca" | "1d76db" | "89d2fc" | "00bcd4" | "b3f4f4" => "🔵",

        // Purples / Violets / Magentas
        "5319e7" | "d4c5f9" | "612d6d" | "7057ff" | "d876e3" | "8e44ad" | "bf55ec" => "🟣",

        // Browns
        "8b572a" | "c4a661" | "bf8c60" => "🟤",

        // Greys / Blacks
        "24292e" | "000000" | "1c1e21" | "333333" | "444444" => "⚫️",

        // Default / Fallback for unknown colors
        _ => "⚪️",
    }
}

/// Serializes a `CallbackAction` to a JSON string. Used for keyboard buttons.
/// expect is ok because inputs are simple and controlled.
pub fn serialize_action(action: &CallbackAction) -> String {
    serde_json::to_string(action).expect("Failed to serialize action")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_color_to_emoji() {
        assert_eq!(github_color_to_emoji("b60205"), "🔴");
        assert_eq!(github_color_to_emoji("f29513"), "🟠");
        assert_eq!(github_color_to_emoji("fef2c0"), "🟡");
        assert_eq!(github_color_to_emoji("0e8a16"), "🟢");
        assert_eq!(github_color_to_emoji("0052cc"), "🔵");
        assert_eq!(github_color_to_emoji("5319e7"), "🟣");
        assert_eq!(github_color_to_emoji("8b572a"), "🟤");
        assert_eq!(github_color_to_emoji("24292e"), "⚫️");
        assert_eq!(github_color_to_emoji("unknown"), "⚪️");
    }

    #[test]
    fn test_serialize_action() {
        let action = CallbackAction::CmdHelp;
        let serialized = serialize_action(&action);
        assert_eq!(serialized, r#""cmd-help""#);
    }
}
