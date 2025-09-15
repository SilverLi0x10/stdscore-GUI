use anyhow::{anyhow, Context, Result};
use eframe::{egui, App, Frame};
use egui::{FontData, FontDefinitions, FontFamily};
use egui_extras::{Column, TableBuilder};
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::{env, fs};

// ---------- Ported from Python Script: Name Configuration ----------

static NAME_PATCHES: Lazy<BTreeMap<&'static str, &'static str>> = Lazy::new(|| {
    BTreeMap::from([
        ("!CQYC-dw", "dw"),
        ("0-CQYC-wht", "wht"),
        ("4_cqyc-lty", "lty"),
        ("谭笑儒_CuO", "谭笑儒"),
        ("刘芸溪1", "刘芸溪"),
        ("李楷瑞_exam", "李楷瑞"),
        ("tangjunxi", "tjx"),
    ])
});

static NAME_REFERENCES: Lazy<BTreeMap<&'static str, &'static str>> = Lazy::new(|| {
    BTreeMap::from([
        ("wht", "王鸿天"), ("pyy", "彭悠扬"), ("dw", "但未"),
        ("czy", "陈泽语"), ("zjx", "张锦轩"), ("wyd", "王宥丁"),
        ("nr", "倪锐"), ("whz", "吴昊臻"), ("lcc", "李承灿"),
        ("szy", "沈子益"), ("hxr", "黄湘瑞"), ("zp", "曾普"),
        ("syc", "沈钰宸"), ("lty", "刘天予"), ("xys", "邢耘硕"),
        ("fzx", "冯泽鑫"), ("tjx", "唐浚希"), ("ljh", "廖俊豪"),
        ("crz", "曹瑞之"), ("zqh", "张勤浩"),
    ])
});

// Regex ported from name_formats in Python
static NAME_FORMAT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^((st.*?|61\d-\d\d|\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})_?)?(G202\d(_|-))?(C2|C3)?(_|-)?((CQYC|cqyc) ?(_|-) ?)?(.*?)(\(.*?\))?$").unwrap()
});

static NAME_IGNORES: Lazy<BTreeSet<&'static str>> = Lazy::new(|| {
    BTreeSet::from([
        "std", "吴戈", "黄彦涛", "李源洲", "黄宇哲", "杨维昊", "邓家昕",
        "舒显航", "杨浩然", "蒋荣杰", "刘浩", "扬淮楠", "陈思艳", "黄笛飞",
        "韩金东", "鲜翔羽", "钟骏宇", "曾彦博", "杨淮楠", "彭越寒", "吴桐雨",
        "任奔奔", "李旻粲", "刘东林",
    ])
});


/// Ported name formatting logic from the Python script.
fn format_name(raw_name: &str) -> String {
    let mut name = raw_name.trim().to_string();

    // Try regex matching first
    if let Some(caps) = NAME_FORMAT_REGEX.captures(&name) {
        if let Some(matched_group) = caps.get(10) {
            let mut extracted = matched_group.as_str().to_string();
            // Check patches
            if let Some(patched) = NAME_PATCHES.get(extracted.as_str()) {
                extracted = patched.to_string();
            }
            // Check references
            return NAME_REFERENCES
                .get(extracted.to_lowercase().as_str())
                .map(|s| s.to_string())
                .unwrap_or(extracted);
        }
    }
    
    // Fallback for non-regex matches
    if let Some(patched) = NAME_PATCHES.get(name.as_str()) {
        name = patched.to_string();
    }
    
    NAME_REFERENCES
        .get(name.to_lowercase().as_str())
        .map(|s| s.to_string())
        .unwrap_or(name)
}


// ---------- Core Application Structs ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersonEntry {
    name: String,
    raw_score: f32,
}

#[derive(Debug, Clone)]
struct FileResult {
    file_label: String,
    people: Vec<PersonEntry>,
    highest_non_std: f32,
}

#[derive(Debug, Default)]
struct AppState {
    file_order: Vec<String>,
    per_file: BTreeMap<String, FileResult>,
    all_people: BTreeSet<String>,
    status: String,
    precision: usize,
}

impl AppState {
    fn new() -> Self {
        Self {
            precision: 2,
            ..Default::default()
        }
    }

    fn add_file(&mut self, label: String, bytes: Vec<u8>) -> Result<()> {
        let html = String::from_utf8(bytes).context("The file is not UTF-8 encoded")?;
        let mut parsed = parse_people_from_html(&html).context("Failed to parse HTML")?;
        
        // Filter out ignored names BEFORE any calculations
        parsed.retain(|p| !NAME_IGNORES.contains(p.name.as_str()));
        
        if parsed.is_empty() {
             return Err(anyhow!("No valid student data found in the file after filtering ignored names."));
        }

        let highest = parsed
            .iter()
            // The python script ignored 'std', but we filter all ignored names now.
            // This filter ensures we only consider non-ignored students for the max score.
            .map(|p| p.raw_score)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        if highest == 0.0 {
            self.status = format!("Warning: Highest score in '{}' is 0. Standard scores may be incorrect.", label);
        }

        if !self.per_file.contains_key(&label) {
            self.file_order.push(label.clone());
        }

        self.all_people
            .extend(parsed.iter().map(|p| p.name.clone()));
        self.per_file.insert(
            label.clone(),
            FileResult {
                file_label: label,
                people: parsed,
                highest_non_std: highest,
            },
        );

        Ok(())
    }

    fn clear(&mut self) {
        *self = AppState::new();
    }
}

// ---------- HTML Parsing (Now more robust) ----------
fn parse_people_from_html(html: &str) -> Result<Vec<PersonEntry>> {
    let doc = Html::parse_document(html);

    // Simplified selector: find the first table in the document.
    let table_sel = Selector::parse("table").unwrap();
    let table = doc
        .select(&table_sel)
        .next()
        .ok_or_else(|| anyhow!("No <table> element found in the HTML document"))?;

    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();
    
    let mut people = Vec::new();
    let mut rows = table.select(&tr_sel);

    // Skip table header
    rows.next(); 

    // Regex to find the first floating point number in a string.
    let re_num = Regex::new(r"-?\d+(\.\d+)?").unwrap();

    for tr in rows {
        let tds: Vec<_> = tr.select(&td_sel).collect();
        if tds.len() < 3 { // Rank, Name, Score
            continue;
        }

        // Column 2: Name
        let name_td = &tds[1];
        let raw_name = if let Some(a) = name_td.select(&a_sel).next() {
            a.text().collect::<String>()
        } else {
            name_td.text().collect::<String>()
        };
        
        let name = format_name(raw_name.trim());
        if name.is_empty() {
            continue;
        }

        // Column 3: Score
        let score_td = &tds[2];
        let score_text = score_td.text().collect::<String>();
        
        let raw_score: f32 = match re_num.find(&score_text) {
            Some(score_match) => score_match.as_str().parse().unwrap_or(0.0),
            None => 0.0, // Default to 0 if no number found
        };
        
        people.push(PersonEntry { name, raw_score });
    }

    if people.is_empty() {
        Err(anyhow!("No people were parsed from the table"))
    } else {
        Ok(people)
    }
}

// ---------- GUI Implementation ----------

struct StdScoreApp {
    state: AppState,
}

impl Default for StdScoreApp {
    fn default() -> Self {
        Self {
            state: AppState::new(),
        }
    }
}

impl App for StdScoreApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("std score calculator (drag in one or more HTML files)");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Accuracy:");
                ui.add(egui::DragValue::new(&mut self.state.precision).clamp_range(0..=6));
                if ui.button("Clear").clicked() {
                    self.state.clear();
                }
                if ui.button("Open File...").clicked() {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("HTML", &["html", "htm"])
                        .pick_files()
                    {
                        for path in files {
                            if let Err(e) = load_path_into_state(&path, &mut self.state) {
                                self.state.status = format!("Loading failed {}: {e}", path.display());
                            }
                        }
                    }
                }
            });
            if !self.state.status.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.state.status);
            }
             ui.label("Rule: The highest score from a non-ignored student is the full score. Std Score = (Raw Score / Full Score) * 100.");
        });

        // Handle file drop
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.state.status.clear(); // Clear old status on new drop
            }
            for dropped in &i.raw.dropped_files {
                if let Some(path) = &dropped.path {
                     if let Err(e) = load_path_into_state(path, &mut self.state) {
                        self.state.status = format!("Loading failed {}: {e}", path.display());
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.per_file.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Drag & drop HTML files here, or click 'Open File...'");
                });
                return;
            }
            draw_table(ui, &self.state);
        });
    }
}

fn load_path_into_state(path: &PathBuf, state: &mut AppState) -> Result<()> {
    let label = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    let bytes = std::fs::read(path)?;
    state.add_file(label, bytes)?;
    Ok(())
}

struct PersonScore {
    name: String,
    avg_std: f32,
    scores: Vec<Option<(f32, f32)>>, // (std_score, raw_score)
}

fn draw_table(ui: &mut egui::Ui, state: &AppState) {
    let name_max_width = state
        .all_people
        .iter()
        .map(|name| {
            ui.fonts(|f| {
                f.layout_no_wrap(
                    name.to_string(),
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                )
                .size()
                .x
            })
        })
        .fold(80.0, f32::max); // Minimum width for name column

    let mut sorted_people: Vec<PersonScore> = state
        .all_people
        .iter()
        .map(|name| {
            let mut std_sum = 0.0f32;
            let mut std_cnt = 0;
            let scores: Vec<Option<(f32, f32)>> = state
                .file_order
                .iter()
                .map(|file| {
                    if let Some((s, raw)) = compute_std_raw_for(&state.per_file, file, name) {
                        std_sum += s;
                        std_cnt += 1;
                        Some((s, raw))
                    } else {
                        None
                    }
                })
                .collect();
            
            let avg_std = if std_cnt > 0 { std_sum / (std_cnt as f32) } else { 0.0 };

            PersonScore {
                name: name.clone(),
                avg_std,
                scores,
            }
        })
        .collect();

    sorted_people.sort_by(|a, b| b.avg_std.partial_cmp(&a.avg_std).unwrap_or(std::cmp::Ordering::Equal));
    
    let mut table_builder = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(name_max_width).resizable(true)) // Name
        .column(Column::initial(80.0).resizable(true));    // Avg Std

    for file in &state.file_order {
        table_builder = table_builder
            .column(Column::initial(80.0).resizable(true)) // Std Score
            .column(Column::initial(80.0).resizable(true)); // Raw Score
    }

    table_builder
        .header(20.0, |mut header| {
            header.col(|ui| { ui.strong("Name"); });
            header.col(|ui| { ui.strong("Avg Std"); });
            for file in &state.file_order {
                header.col(|ui| { ui.strong(file.replace(".html", "")); });
                header.col(|ui| { ui.strong("Raw"); });
            }
        })
        .body(|mut body| {
            for p_score in &sorted_people {
                body.row(20.0, |mut row| {
                    row.col(|ui| { ui.label(&p_score.name); });
                    row.col(|ui| { ui.label(format!("{:.*}", state.precision, p_score.avg_std)); });
                    for score in &p_score.scores {
                        if let Some((std, raw)) = score {
                            row.col(|ui| { ui.label(format!("{:.*}", state.precision, std)); });
                            row.col(|ui| { ui.label(format!("{}", raw)); });
                        } else {
                            row.col(|ui| { ui.label("-"); });
                            row.col(|ui| { ui.label("-"); });
                        }
                    }
                });
            }
        });
}

fn compute_std_raw_for(
    per_file: &BTreeMap<String, FileResult>,
    file: &str,
    name: &str,
) -> Option<(f32, f32)> {
    let fr = per_file.get(file)?;
    let pe = fr.people.iter().find(|p| p.name == name)?;
    let raw = pe.raw_score;
    let std_score = if fr.highest_non_std > 0.0 {
        (raw / fr.highest_non_std) * 100.0
    } else {
        0.0
    };
    Some((std_score, raw))
}


fn setup_chinese_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    // Use a known common font first
    fonts.font_data.insert(
        "msyh".to_owned(),
        FontData::from_static(include_bytes!("c:/Windows/Fonts/msyh.ttc")),
    );
    // Prioritize this font for proportional (normal) and monospace text
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "msyh".to_owned());
    fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "msyh".to_owned());
    ctx.set_fonts(fonts);
}


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "std score calculator",
        options,
        Box::new(|cc| {
            // Note: For font loading to work, you might need to adjust the path or embed the font.
            // This example assumes a standard Windows installation path for `msyh.ttc`.
            // setup_chinese_fonts(&cc.egui_ctx);
            Box::new(StdScoreApp::default())
        }),
    )
}