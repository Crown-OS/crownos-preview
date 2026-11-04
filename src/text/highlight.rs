use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const DARK_THEME: &str = "base16-ocean.dark";
const LIGHT_THEME: &str = "InspiredGitHub";
const TAB: &str = "    ";

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub rgb: [u8; 3],
    pub text: String,
}

pub type Line = Vec<Span>;

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn themes() -> &'static ThemeSet {
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    THEMES.get_or_init(ThemeSet::load_defaults)
}

/// `source` split into colored spans per line, by the syntax its `extension` names.
pub fn highlight(source: &str, extension: &str, dark: bool) -> Vec<Line> {
    let syntaxes = syntaxes();
    let syntax = syntaxes
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let theme = &themes().themes[if dark { DARK_THEME } else { LIGHT_THEME }];
    let mut highlighter = HighlightLines::new(syntax, theme);
    LinesWithEndings::from(source)
        .map(|line| match highlighter.highlight_line(line, syntaxes) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| Span {
                    rgb: [style.foreground.r, style.foreground.g, style.foreground.b],
                    text: printable(text),
                })
                .filter(|span| !span.text.is_empty())
                .collect(),
            Err(_) => vec![Span {
                rgb: [128, 128, 128],
                text: printable(line),
            }],
        })
        .collect()
}

fn printable(text: &str) -> String {
    text.trim_end_matches(['\n', '\r']).replace('\t', TAB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_line_becomes_one_line_of_spans() {
        let lines = highlight("fn main() {\n\tlet x = 1;\n}\n", "rs", true);
        assert_eq!(lines.len(), 3);
        let second: String = lines[1].iter().map(|span| span.text.as_str()).collect();
        assert_eq!(second, "    let x = 1;");
        assert!(lines[0].len() > 1, "keywords are colored apart from names");
    }
}
