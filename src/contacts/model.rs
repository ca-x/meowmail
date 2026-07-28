use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub notes: String,
    pub search_aliases: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(crate) fn contact_search_aliases(display_name: &str) -> Vec<String> {
    let mut pinyin_full = String::new();
    let mut pinyin_words = Vec::new();
    let mut pinyin_initials = String::new();
    let mut english_initials = String::new();
    let mut mixed_initials = String::new();
    let mut in_latin_word = false;

    for character in display_name.chars() {
        if let Some(pinyin) = character.to_pinyin() {
            let plain = pinyin.plain();
            let initial = pinyin.first_letter();
            pinyin_full.push_str(plain);
            pinyin_words.push(plain);
            pinyin_initials.push_str(initial);
            mixed_initials.push_str(initial);
            in_latin_word = false;
        } else if character.is_alphanumeric() {
            if !in_latin_word {
                for lower in character.to_lowercase() {
                    english_initials.push(lower);
                    mixed_initials.push(lower);
                }
            }
            in_latin_word = true;
        } else {
            in_latin_word = false;
        }
    }

    let mut aliases = Vec::new();
    push_alias(&mut aliases, pinyin_full);
    push_alias(&mut aliases, pinyin_words.join(" "));
    push_alias(&mut aliases, pinyin_initials);
    push_alias(&mut aliases, english_initials);
    push_alias(&mut aliases, mixed_initials);
    aliases
}

fn push_alias(aliases: &mut Vec<String>, alias: String) {
    if !alias.is_empty() && !aliases.contains(&alias) {
        aliases.push(alias);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub display_name: String,
    pub email: String,
    #[serde(default)]
    pub notes: String,
}

impl ContactInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.display_name = clean_optional(&self.display_name, "display name", 160)?;
        self.email = clean_required(&self.email, "email", 254)?.to_ascii_lowercase();
        if !looks_like_email(&self.email) {
            return Err(AppError::Validation("contact email is invalid".into()));
        }
        self.notes = clean_optional(&self.notes, "notes", 4096)?;
        if self.display_name.is_empty() {
            self.display_name = self.email.clone();
        }
        Ok(())
    }
}

fn clean_required(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}

fn clean_optional(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.chars().count() > max || value.chars().any(|ch| ch == '\0') {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !value.contains(['\r', '\n', ' ', '<', '>'])
}

#[cfg(test)]
mod tests {
    use super::contact_search_aliases;

    #[test]
    fn builds_full_pinyin_and_initial_aliases() {
        let aliases = contact_search_aliases("张三");
        assert!(aliases.iter().any(|alias| alias == "zhangsan"));
        assert!(aliases.iter().any(|alias| alias == "zhang san"));
        assert!(aliases.iter().any(|alias| alias == "zs"));
    }

    #[test]
    fn builds_english_and_mixed_name_initials() {
        assert!(
            contact_search_aliases("John Smith")
                .iter()
                .any(|alias| alias == "js")
        );
        assert!(
            contact_search_aliases("张 San")
                .iter()
                .any(|alias| alias == "zs")
        );
    }
}
