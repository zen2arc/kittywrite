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
    style_key: u64,
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
    pub fn highlight(
        &self,
        code: &str,
        language: &str,
        font_size: f32,
        line_height: f32,
    ) -> LayoutJob {
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        let content_hash = hasher.finish();
        let style_key = ((font_size.to_bits() as u64) << 32) | (line_height.to_bits() as u64);

        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.iter().find(|e| {
                e.content_hash == content_hash && e.language == language && e.style_key == style_key
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
            None => return plain_job(code, font_size, line_height),
        };

        let effective_lh = if (line_height - 1.0).abs() < 0.01 {
            None
        } else {
            Some(font_size * line_height)
        };

        let mut hl = HighlightLines::new(syntax, theme);
        let mut job = LayoutJob::default();
        let font = FontId::monospace(font_size);

        for line in LinesWithEndings::from(code) {
            let spans = match hl.highlight_line(line, &self.ss) {
                Ok(s) => s,
                Err(_) => {
                    job.append(line, 0.0, plain_format(&font, effective_lh));
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
                        line_height: effective_lh,
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
                style_key,
                job: job.clone(),
            });
        }

        job
    }
}

pub fn apply_match_highlights(
    job: &mut LayoutJob,
    matches: &[usize],
    current_idx: usize,
    query_len: usize,
) {
    if matches.is_empty() || query_len == 0 {
        return;
    }
    let old_sections = std::mem::take(&mut job.sections);
    let mut new_sections: Vec<egui::text::LayoutSection> =
        Vec::with_capacity(old_sections.len() + matches.len() * 2);

    for section in &old_sections {
        split_section(section, matches, current_idx, query_len, &mut new_sections);
    }

    job.sections = new_sections;
}

fn split_section(
    section: &egui::text::LayoutSection,
    matches: &[usize],
    current_idx: usize,
    query_len: usize,
    out: &mut Vec<egui::text::LayoutSection>,
) {
    let s_start = section.byte_range.start;
    let s_end = section.byte_range.end;
    let mut cursor = s_start;
    let mut is_first = true;

    for (match_idx, &m_start) in matches.iter().enumerate() {
        let m_end = m_start + query_len;
        if m_end <= s_start || m_start >= s_end {
            continue;
        }
        let seg_start = m_start.max(s_start);
        let seg_end = m_end.min(s_end);

        if cursor < seg_start {
            out.push(egui::text::LayoutSection {
                leading_space: if is_first { section.leading_space } else { 0.0 },
                byte_range: cursor..seg_start,
                format: section.format.clone(),
            });
            is_first = false;
        }

        let mut fmt = section.format.clone();
        if match_idx == current_idx {
            fmt.background = Color32::from_rgb(200, 150, 20);
            fmt.color = Color32::WHITE;
        } else {
            fmt.background = Color32::from_rgba_premultiplied(70, 70, 160, 120);
        }
        out.push(egui::text::LayoutSection {
            leading_space: if is_first { section.leading_space } else { 0.0 },
            byte_range: seg_start..seg_end,
            format: fmt,
        });
        is_first = false;
        cursor = seg_end;
    }

    if cursor < s_end {
        out.push(egui::text::LayoutSection {
            leading_space: if is_first { section.leading_space } else { 0.0 },
            byte_range: cursor..s_end,
            format: section.format.clone(),
        });
    }
}

fn plain_job(code: &str, font_size: f32, line_height: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let lh = if (line_height - 1.0).abs() < 0.01 {
        None
    } else {
        Some(font_size * line_height)
    };
    job.append(code, 0.0, plain_format(&FontId::monospace(font_size), lh));
    job
}

pub fn plain_highlight(code: &str, font_size: f32, line_height: f32) -> LayoutJob {
    plain_job(code, font_size, line_height)
}

fn plain_format(font: &FontId, line_height: Option<f32>) -> TextFormat {
    TextFormat {
        font_id: font.clone(),
        line_height,
        color: Color32::from_rgb(200, 195, 215),
        ..Default::default()
    }
}
