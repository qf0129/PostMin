//! `{{variable}}` substitution for request fields.

use crate::model::{Environment, EnvironmentVariable};

/// Replace `{{name}}` tokens with values from the active environment.
///
/// Unknown or malformed tokens are left untouched so the user can see what was not resolved.
pub fn resolve(template: &str, environment: Option<&Environment>) -> String {
    let Some(environment) = environment else {
        return template.to_string();
    };
    if !template.contains("{{") {
        return template.to_string();
    }

    let mut output = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            output.push_str(&rest[start..]);
            return output;
        };

        let name = after_start[..end].trim();
        let token_end = start + 2 + end + 2;
        if name.is_empty() || name.contains('{') || name.contains('}') {
            output.push_str(&rest[start..token_end]);
        } else if let Some(variable) = lookup(environment, name) {
            output.push_str(&variable.value);
        } else {
            output.push_str(&rest[start..token_end]);
        }
        rest = &rest[token_end..];
    }

    output.push_str(rest);
    output
}

fn lookup<'a>(environment: &'a Environment, name: &str) -> Option<&'a EnvironmentVariable> {
    environment
        .variables
        .iter()
        .find(|variable| variable.name.trim() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> Environment {
        let mut environment = Environment::new("test");
        environment
            .variables
            .push(EnvironmentVariable::new("host", "https://example.com", ""));
        environment
            .variables
            .push(EnvironmentVariable::new("token", "abc-123", ""));
        environment
    }

    #[test]
    fn replaces_known_variables() {
        let environment = environment();
        assert_eq!(
            resolve("{{host}}/users?token={{token}}", Some(&environment)),
            "https://example.com/users?token=abc-123"
        );
    }

    #[test]
    fn trims_spaces_inside_braces() {
        let environment = environment();
        assert_eq!(
            resolve("{{ host }}/{{  token  }}", Some(&environment)),
            "https://example.com/abc-123"
        );
    }

    #[test]
    fn keeps_unknown_and_malformed_tokens() {
        let environment = environment();
        assert_eq!(
            resolve(
                "{{host}}/{{missing}}/{{}}/{{not valid}}",
                Some(&environment)
            ),
            "https://example.com/{{missing}}/{{}}/{{not valid}}"
        );
        assert_eq!(resolve("plain text", Some(&environment)), "plain text");
        assert_eq!(resolve("{{host}}", None), "{{host}}");
    }

    #[test]
    fn does_not_touch_json_braces() {
        let environment = environment();
        let body = r#"{"name":"PostMin","nested":{"ok":true}}"#;
        assert_eq!(resolve(body, Some(&environment)), body);
    }

    #[test]
    fn first_duplicate_name_wins() {
        let mut environment = environment();
        environment.variables.push(EnvironmentVariable::new(
            "host",
            "https://second.example",
            "",
        ));
        assert_eq!(
            resolve("{{host}}", Some(&environment)),
            "https://example.com"
        );
    }
}
