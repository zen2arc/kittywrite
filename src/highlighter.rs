use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Stroke};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

struct CacheEntry {
    content_hash: u64,
    language: String,
    font_bits: u32,
    job: LayoutJob,
}

pub struct Highlighter {
    ss: SyntaxSet,
    ts: ThemeSet,
    cache: Mutex<Vec<CacheEntry>>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self {
            ss: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
            cache: Mutex::new(Vec::new()),
        }
    }
}

impl Highlighter {
    pub fn highlight(&self, code: &str, language: &str, font_size: f32) -> LayoutJob {
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        let content_hash = hasher.finish();
        let font_bits = font_size.to_bits();

        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.iter().find(|e| {
                e.content_hash == content_hash
                    && e.language == language
                    && e.font_bits == font_bits
            }) {
                return entry.job.clone();
            }
        }

        let syntax = self
            .ss
            .find_syntax_by_name(language)
            .or_else(|| self.ss.find_syntax_by_extension(language))
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());

        let preferred = ["base16-ocean.dark", "base16-mocha.dark", "Monokai"];
        let theme = preferred
            .iter()
            .find_map(|name| self.ts.themes.get(*name))
            .or_else(|| self.ts.themes.values().next());

        let theme = match theme {
            Some(t) => t,
            None => return plain_job(code, font_size),
        };

        let mut hl = HighlightLines::new(syntax, theme);
        let mut job = LayoutJob::default();
        let font = FontId::monospace(font_size);

        for line in LinesWithEndings::from(code) {
            let spans = match hl.highlight_line(line, &self.ss) {
                Ok(s) => s,
                Err(_) => {
                    job.append(line, 0.0, plain_format(&font));
                    continue;
                }
            };

            for (style, text) in spans {
                let fg = style.foreground;
                let color = Color32::from_rgb(fg.r, fg.g, fg.b);
                let italic = style.font_style.contains(FontStyle::ITALIC);
                let under = style.font_style.contains(FontStyle::UNDERLINE);

                job.append(
                    text,
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color,
                        italics: italic,
                        underline: if under {
                            Stroke::new(1.0, color)
                        } else {
                            Stroke::NONE
                        },
                        ..Default::default()
                    },
                );
            }
        }

        {
            let mut cache = self.cache.lock().unwrap();
            if cache.len() >= 8 {
                cache.remove(0);
            }
            cache.push(CacheEntry {
                content_hash,
                language: language.to_string(),
                font_bits,
                job: job.clone(),
            });
        }

        job
    }
}

fn plain_job(code: &str, font_size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(code, 0.0, plain_format(&FontId::monospace(font_size)));
    job
}

fn plain_format(font: &FontId) -> TextFormat {
    TextFormat {
        font_id: font.clone(),
        color: Color32::from_rgb(200, 195, 215),
        ..Default::default()
    }
}
