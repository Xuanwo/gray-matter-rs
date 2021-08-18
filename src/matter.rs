use crate::engine::Engine;
use crate::entity::{ParsedEntity, ParsedEntityStruct};
use regex::Regex;

enum Part {
    Matter,
    MaybeExcerpt,
    Content,
}

pub struct Matter<T: Engine> {
    pub delimiter: String,
    pub excerpt_delimiter: Option<String>,
    engine: T,
}

impl<T: Engine> Matter<T> {
    pub fn new() -> Self {
        return Matter {
            delimiter: "---".to_string(),
            excerpt_delimiter: None,
            engine: T::new(),
        };
    }

    /// **matter_struct** takes a &str, extracts and parses front-matter from it,
    /// then returns a ParsedEntityStruct result.
    ///
    /// ```rust
    /// # use gray_matter::matter::Matter;
    /// # use gray_matter::engine::yaml::YAML;
    /// # use gray_matter::entity::ParsedEntityStruct;
    /// #[derive(serde::Deserialize)]
    /// struct Config {
    ///     title: String,
    /// }
    /// let matter: Matter<YAML> = Matter::new();
    /// let input = "---\ntitle: Home\n---\nOther stuff".to_string();
    /// let parsed_entity: ParsedEntityStruct<Config> =  matter.matter_struct(input);
    /// ```
    pub fn parse_with_struct<D: serde::de::DeserializeOwned>(
        &self,
        input: String,
    ) -> Option<ParsedEntityStruct<D>> {
        let parsed_entity = self.parse(input.clone())?;
        let data: D = parsed_entity.data?.deserialize().ok()?;

        Some(ParsedEntityStruct {
            data,
            content: parsed_entity.content,
            excerpt: parsed_entity.excerpt,
            orig: parsed_entity.orig,
            matter: parsed_entity.matter,
        })
    }

    /// **parse** takes a `&str`, extracts and parses front-matter from it,
    /// then returns a ParsedEntity result.
    ///
    /// ```rust
    /// # use gray_matter::matter::Matter;
    /// # use gray_matter::engine::yaml::YAML;
    /// let matter: Matter<YAML> = Matter::new();
    /// let input = "---\ntitle: Home\n---\nOther stuff".to_string();
    /// let parsed_entity =  matter.parse(input);
    /// ```
    pub fn parse(&self, input: String) -> Option<ParsedEntity> {
        // Check if input is empty or shorter than the delimiter
        if input.is_empty()
        || input.len() <= self.delimiter.len()
        {
            return None;
        }

        // Initialize ParsedEntity
        let mut parsed_entity = ParsedEntity {
            data: None,
            excerpt: None,
            content: String::new(),
            orig: input.clone(),
            matter: String::new(),
        };

        // If excerpt delimiter is given, use it. Otherwise, use normal delimiter
        let excerpt_delimiter = self.excerpt_delimiter.clone()
            .unwrap_or_else(|| self.delimiter.clone());

        let mut lines = input.lines();

        // If first line starts with a delimiter followed by newline, we are looking at front
        // matter. Else, we might be looking at an excerpt.
        // FIXME: We are only trimming the start of the line. We might have a delimiter with
        // whitespace after: `---  \n`. gray_matter should handle this.
        let mut looking_at = if input.trim_start().starts_with(&(self.delimiter.clone() + "\n")) {
            lines.next();
            Part::Matter
        } else { Part::MaybeExcerpt };

        let mut acc = String::new();
        for line in lines {
            &line.to_string().push('\n');
            acc += &format!("\n{}", line);
            match looking_at {
                Part::Matter => {
                    if line.trim() == self.delimiter {
                        let comment_re = Regex::new(r"(?m)^\s*#[^\n]+").unwrap();
                        let matter = comment_re.replace_all(&acc, "")
                            .trim()
                            .strip_suffix(&self.delimiter).unwrap()
                            .trim_matches('\n')
                            .to_string();

                        if !matter.is_empty() {
                            parsed_entity.data = Some(self.engine.parse(&matter));
                            parsed_entity.matter = matter;
                        }

                        acc = String::new();
                        looking_at = Part::MaybeExcerpt;
                    }
                },

                Part::MaybeExcerpt => {
                    if line.trim() == excerpt_delimiter {
                        parsed_entity.excerpt = Some(
                            acc.trim()
                            .strip_prefix(&self.delimiter).unwrap_or(&acc)
                            .strip_suffix(&excerpt_delimiter).unwrap()
                            .to_string()
                        );

                        acc = String::new();
                        looking_at = Part::Content;
                    }
                },

                Part::Content => {}
            }
        }

        parsed_entity.content = acc.trim().to_string();

        Some(parsed_entity)
    }
}
