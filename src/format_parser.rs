use std::collections::HashMap;
use std::env;

/// Parses a format string with variable substitution.
///
/// Expressions in curly braces reference one of the variables or a process-scoped environment variable (when prefixed with env:).
///
/// Examples:
/// - `{Major}.{Minor}.{Patch}.{WeightedPreReleaseNumber ?? 0}`: Use a variable if non-null or a fallback value otherwise
/// - `{Major}.{Minor}.{Patch}.{env:BUILD_NUMBER}`: Use an environment variable or raise an error if not available
/// - `{Major}.{Minor}.{Patch}.{env:BUILD_NUMBER ?? 42}`: Use an environment variable if available or a fallback value otherwise
pub fn parse_format_string(
    format: &str,
    variables: &HashMap<String, String>,
) -> Result<String, String> {
    let mut result = String::new();
    let mut current_pos = 0;

    while current_pos < format.len() {
        // Find the next opening brace
        if let Some(start) = format[current_pos..].find('{') {
            // Add the text before the opening brace to the result
            result.push_str(&format[current_pos..current_pos + start]);
            current_pos += start;

            // Find the closing brace
            if let Some(end) = format[current_pos..].find('}') {
                // Extract the expression inside the braces
                let expr = &format[current_pos + 1..current_pos + end];

                // Parse the expression
                match parse_expression(expr, variables) {
                    Ok(value) => result.push_str(&value),
                    Err(e) => return Err(e),
                }

                current_pos += end + 1;
            } else {
                return Err(format!("Unclosed brace in format string: {}", format));
            }
        } else {
            // No more opening braces, add the rest of the format string to the result
            result.push_str(&format[current_pos..]);
            break;
        }
    }

    Ok(result)
}

/// Parses an expression inside curly braces.
fn parse_expression(expr: &str, variables: &HashMap<String, String>) -> Result<String, String> {
    // Check if the expression has a fallback value
    if let Some(pos) = expr.find("??") {
        let var_name = expr[..pos].trim();
        let fallback = expr[pos + 2..].trim();

        // Try to get the variable value
        match get_variable_value(var_name, variables) {
            Ok(value) => Ok(value),
            Err(_) => {
                // If the variable is not found, use the fallback value
                Ok(fallback.to_string())
            }
        }
    } else {
        // No fallback value, just get the variable value
        get_variable_value(expr.trim(), variables)
    }
}

/// Gets the value of a variable.
fn get_variable_value(
    var_name: &str,
    variables: &HashMap<String, String>,
) -> Result<String, String> {
    // Check if it's an environment variable
    if var_name.starts_with("env:") {
        let env_var = &var_name[4..];
        match env::var(env_var) {
            Ok(value) => Ok(value),
            Err(_) => Err(format!("Environment variable not found: {}", env_var)),
        }
    } else {
        // Look up the variable in the provided map
        match variables.get(var_name) {
            Some(value) => Ok(value.clone()),
            None => Err(format!("Variable not found: {}", var_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format_string_with_variables() {
        let mut variables = HashMap::new();
        variables.insert("Major".to_string(), "1".to_string());
        variables.insert("Minor".to_string(), "2".to_string());
        variables.insert("Patch".to_string(), "3".to_string());
        variables.insert("WeightedPreReleaseNumber".to_string(), "4".to_string());

        let format = "{Major}.{Minor}.{Patch}.{WeightedPreReleaseNumber}";
        let result = parse_format_string(format, &variables).unwrap();
        assert_eq!(result, "1.2.3.4");
    }

    #[test]
    fn test_parse_format_string_with_fallback() {
        let mut variables = HashMap::new();
        variables.insert("Major".to_string(), "1".to_string());
        variables.insert("Minor".to_string(), "2".to_string());
        variables.insert("Patch".to_string(), "3".to_string());

        let format = "{Major}.{Minor}.{Patch}.{WeightedPreReleaseNumber ?? 0}";
        let result = parse_format_string(format, &variables).unwrap();
        assert_eq!(result, "1.2.3.0");
    }

    #[test]
    fn test_parse_format_string_with_env_var() {
        let variables = HashMap::new();

        // Set an environment variable for testing
        unsafe {
            env::set_var("TEST_BUILD_NUMBER", "42");
        }

        let format = "{env:TEST_BUILD_NUMBER}";
        let result = parse_format_string(format, &variables).unwrap();
        assert_eq!(result, "42");

        // Clean up
        unsafe {
            env::remove_var("TEST_BUILD_NUMBER");
        }
    }

    #[test]
    fn test_parse_format_string_with_env_var_fallback() {
        let variables = HashMap::new();

        let format = "{env:NONEXISTENT_VAR ?? 42}";
        let result = parse_format_string(format, &variables).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_parse_format_string_with_missing_var_no_fallback() {
        let variables = HashMap::new();

        let format = "{Major}";
        let result = parse_format_string(format, &variables);
        assert!(result.is_err());
    }
}
