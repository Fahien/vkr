// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

pub trait Camelcase {
    /// Converts a string to camelcase, removing all `-` and `_` characters.
    fn to_camelcase(self) -> String;
}

impl Camelcase for &str {
    fn to_camelcase(self) -> String {
        let (symbol_indices, _): (Vec<usize>, Vec<char>) = self
            .chars()
            .enumerate()
            .filter(|(_, c)| *c == '-' || *c == '_')
            .unzip();

        let mut name = self.to_string();
        name.replace_range(0..1, &name[0..1].to_uppercase());

        for i in symbol_indices {
            if i < name.len() - 1 {
                let char = name.chars().nth(i + 1).unwrap().to_uppercase().to_string();
                name.replace_range(i + 1..i + 2, &char);
            }
        }

        let name = name.chars().filter(|&c| c != '_' && c != '-').collect();

        name
    }
}

/// This function returns the prefix of `name`, which is the
/// name of a shader function without its ending with vs or fs
pub fn get_prefix(name: &str) -> String {
    let last_underscore_position = name.len()
        - name
            .chars()
            .rev()
            .position(|c| c == '_')
            .expect("Failed to get prefix");

    name[..last_underscore_position - 1].to_string()
}

#[test]
fn test_get_prefix() {
    let prefix = get_prefix("main_method_vs");
    assert!(prefix == "main_method");
}

/// Returns `Some(v)` with the inner value of an enum or `None`
/// ```rust
/// inner_value!(enum_value, EnumType(value) => value);
/// ```
macro_rules! inner_value {
    ($value:expr, $pattern:pat => $extracted_value:expr) => {
        match $value {
            $pattern => Some($extracted_value),
            _ => None,
        }
    };
}

pub(crate) use inner_value;
