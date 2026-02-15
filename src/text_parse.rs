use rand::Rng;

/// Parses a text string with random substitutions, weighted probabilities, and escape sequences.
///
/// Syntax:
/// - `<option1|option2|option3>` - random choice
/// - `<common:70|rare:30>` - weighted choice (weights are relative)
/// - `\<` and `\>` - escaped angle brackets (literal < and >)
/// - `\|` - escaped pipe (literal | inside options)
pub fn parse_random_text(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut rng = rand::rng();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // Handle escape sequences
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '<' || next_ch == '>' || next_ch == '|' || next_ch == '\\' {
                    result.push(chars.next().unwrap());
                    continue;
                }
            }
            // Not a recognized escape, just include the backslash
            result.push(ch);
        } else if ch == '<' {
            // Collect everything until we find the closing '>'
            let mut options_str = String::new();
            let mut found_closing = false;
            let mut escape_next = false;

            while let Some(&next_ch) = chars.peek() {
                if escape_next {
                    options_str.push(chars.next().unwrap());
                    escape_next = false;
                } else if next_ch == '\\' {
                    chars.next(); // consume the backslash
                    escape_next = true;
                } else if next_ch == '>' {
                    chars.next(); // consume the '>'
                    found_closing = true;
                    break;
                } else {
                    options_str.push(chars.next().unwrap());
                }
            }

            if found_closing && !options_str.is_empty() {
                // Parse options with optional weights
                let choice = select_weighted_option(&options_str, &mut rng);
                result.push_str(&choice);
            } else {
                // Malformed pattern, just include the '<' and what we collected
                result.push('<');
                result.push_str(&options_str);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Selects a weighted option from a string like "option1:weight1|option2:weight2|option3"
fn select_weighted_option<R: Rng>(options_str: &str, rng: &mut R) -> String {
    let mut options = Vec::new();
    let mut weights = Vec::new();

    for part in options_str.split('|') {
        if let Some((text, weight_str)) = part.rsplit_once(':') {
            // Try to parse weight
            if let Ok(weight) = weight_str.trim().parse::<u32>() {
                if weight > 0 {
                    options.push(text.to_string());
                    weights.push(weight);
                    continue;
                }
            }
        }
        // No weight or invalid weight, default to weight of 1
        options.push(part.to_string());
        weights.push(1);
    }

    if options.is_empty() {
        return String::new();
    }

    // Calculate total weight
    let total_weight: u32 = weights.iter().sum();
    let mut roll = rng.random_range(0..total_weight);

    // Select based on weighted probability
    for (i, &weight) in weights.iter().enumerate() {
        if roll < weight {
            return options[i].clone();
        }
        roll -= weight;
    }

    // Fallback (shouldn't reach here)
    options[0].clone()
}
