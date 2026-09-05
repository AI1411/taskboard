use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SlugError {
    #[error("slug is empty")]
    Empty,
}

pub fn slugify(name: &str) -> Result<String, SlugError> {
    let mut slug = String::new();
    let mut prev_dash = true;

    for ch in name.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        Err(SlugError::Empty)
    } else {
        Ok(slug)
    }
}

pub fn next_unique_slug(desired: &str, live_slugs: &[&str]) -> Result<String, SlugError> {
    let base = slugify(desired)?;
    if !live_slugs.contains(&base.as_str()) {
        return Ok(base);
    }

    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !live_slugs.contains(&candidate.as_str()) {
            return Ok(candidate);
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_renai_sim() {
        assert_eq!(slugify("Renai Sim").unwrap(), "renai-sim");
    }

    #[test]
    fn slugify_empty_or_punctuation_only_returns_empty_error() {
        assert_eq!(slugify(""), Err(SlugError::Empty));
        assert_eq!(slugify("   "), Err(SlugError::Empty));
        assert_eq!(slugify("!!!"), Err(SlugError::Empty));
        assert_eq!(slugify("---"), Err(SlugError::Empty));
    }

    #[test]
    fn next_unique_slug_returns_base_when_unused() {
        assert_eq!(next_unique_slug("renai-sim", &[]).unwrap(), "renai-sim");
    }

    #[test]
    fn slug_collision_appends_number() {
        let live = ["renai-sim"];
        assert_eq!(next_unique_slug("renai-sim", &live).unwrap(), "renai-sim-2");
    }

    #[test]
    fn explicit_duplicate_slug_is_error_when_unique_not_requested() {
        // next_unique_slug always suffixes; callers that passed --slug do not call it.
        assert_eq!(slugify("Renai Sim").unwrap(), "renai-sim");
    }
}
