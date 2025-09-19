// Use the GUI subsystem only on Windows
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::{Context, Result, anyhow};
use eframe::{App, Frame, egui};
use egui::{FontData, FontDefinitions, FontFamily};
use egui_extras::{Column, TableBuilder};
use phf::phf_map;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Clone)]
struct PersonEntry {
    name: String,
    raw_score: f32,
}

#[derive(Debug, Clone)]
struct FileResult {
    // file_label: String,
    people: Vec<PersonEntry>,
    highest_non_std: f32,
}

#[derive(Debug, Default)]
struct AppState {
    // Maintain file order to build the list header
    file_order: Vec<String>,
    // Each file name -> parsed result of that file
    per_file: BTreeMap<String, FileResult>,
    // All people's names that appeared (maintain order)
    all_people: BTreeSet<String>,
    // Interface status
    status: String,
    // Decimal display precision
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
        let parsed = parse_people_from_html(&html).context("Failed to parse HTML")?;
        let highest = parsed
            .iter()
            .filter(|p| p.name.to_lowercase() != "std")
            .map(|p| p.raw_score)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        if !self.per_file.contains_key(&label) {
            self.file_order.push(label.clone());
        }

        self.all_people
            .extend(parsed.iter().map(|p| p.name.clone()));
        self.per_file.insert(
            label.clone(),
            FileResult {
                // file_label: label,
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

/*
 * Replacement for name in the table
 * (name -> replacement)
 * where name is LOWERCASE
 */
static REPLACE_NAME: phf::Map<&str, &str> = phf_map!(
    "cqyc-wht" => "CQYC-王鸿天",
);

fn parse_people_from_html(html: &str) -> Result<Vec<PersonEntry>> {
    let doc = Html::parse_document(html);

    // select the third <p> under <body>
    let p_sel = Selector::parse("body > p").unwrap();
    let mut ps = doc.select(&p_sel);
    let p3 = ps
        .nth(2)
        .ok_or_else(|| anyhow!("The third <p> under <body> was not found"))?;

    // find the table under the third <p>
    let table_sel = Selector::parse("table").unwrap();
    let table = p3
        .select(&table_sel)
        .next()
        .ok_or_else(|| anyhow!("<table> not found in 3rd <p>"))?;

    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();

    let mut people = Vec::new();
    let mut rows = table.select(&tr_sel);

    // skip table header (the first row is usually <th>)
    if rows.next().is_none() {
        return Err(anyhow!("The table has no data rows"));
    }

    // extract number (tolerate spaces/colors)
    let re_num = Regex::new(r"(?x) -?\d+(?:\.\d+)? ").unwrap();

    for tr in rows {
        let tds: Vec<_> = tr.select(&td_sel).collect();
        if tds.len() < 3 {
            // at least 3 columns: rank, name, total score
            eprintln!("UNEXPECTED: {:?}", tds);
            continue;
        }

        // name in 2nd column (may be wrapped in <a>)
        let name_td = &tds[1];
        let mut name = if let Some(a) = name_td.select(&a_sel).next() {
            a.text().collect::<String>().trim().to_string()
        } else {
            name_td.text().collect::<String>().trim().to_string()
        };
        if name.is_empty() {
            eprintln!("UNEXPECTED: Empty name column");
            continue;
        }

        // total score in 3rd column, take first number
        let score_td = &tds[2];
        let score_text = score_td.text().collect::<String>();
        let score_str = re_num
            .find(&score_text)
            .ok_or_else(|| {
                anyhow!(
                    "Unable to parse number in total score column (name: {})",
                    name
                )
            })?
            .as_str();
        let raw_score: f32 = score_str
            .parse()
            .with_context(|| format!("score parsing failed: {} (name: {})", score_str, name))?;

        if let Some(new_name) = REPLACE_NAME.get(name.to_lowercase().as_str()) {
            name = new_name.to_string();
        }
        people.push(PersonEntry { name, raw_score });
    }

    if people.is_empty() {
        Err(anyhow!("No one was parsed from the table"))
    } else {
        Ok(people)
    }
}

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
                ui.add(egui::DragValue::new(&mut self.state.precision).range(0..=6));
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
                // --- Push the toggle button to the far right ---
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Toggle Dark/Light").clicked() {
                        if ctx.style().visuals.dark_mode {
                            ctx.set_theme(egui::Theme::Light);
                        } else {
                            ctx.set_theme(egui::Theme::Dark);
                        }
                    }
                });
            });
            if !self.state.status.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.state.status);
            }
            ui.label("Rule: The highest normal score in the file whose name is not 'std' is counted as the full score, std score = normal score / full score * 100.");
        });

        // handle file drop
        ctx.input(|i| {
            for dropped in &i.raw.dropped_files {
                if let Some(bytes) = dropped.bytes.clone() {
                    let label = dropped
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                        .or_else(|| Some(dropped.name.clone()))
                        .unwrap_or_else(|| "dropped.html".to_string());
                    if let Err(e) = self.state.add_file(label, bytes.to_vec()) {
                        self.state.status = format!("Parsing failed: {e}");
                    }
                } else if let Some(path) = dropped.path.clone() {
                    if let Err(e) = load_path_into_state(&path, &mut self.state) {
                        self.state.status = format!("Loading failed {}: {e}", path.display());
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.per_file.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Drag and drop one or more HTML files, or click 'Open File...' to select a file.");
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
    scores: Vec<Option<(f32, f32)>>,
}

fn draw_table(ui: &mut egui::Ui, state: &AppState) {
    // Columns design:
    // Name | Avg Std | [File1 Std] [File1 Raw] | [File2 Std] [File2 Raw] | ...

    // retrieve the FontId corresponding to the current Body style
    let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();

    // Calculate the Name column width
    let name_max_width = state
        .all_people
        .iter()
        .map(|name| {
            ui.fonts(|f| {
                f.layout_no_wrap(name.to_string(), body_font_id.clone(), egui::Color32::WHITE)
                    .rect
                    .width()
            })
        })
        .fold(0.0, f32::max);

    // Calculate file column widths
    let file_widths: Vec<f32> = state
        .file_order
        .iter()
        .map(|fname| {
            ui.fonts(|f| {
                f.layout_no_wrap(fname.clone(), body_font_id.clone(), egui::Color32::WHITE)
                    .rect
                    .width()
            })
        })
        .collect();

    // Wrapped in horizontal scrolling area
    egui::ScrollArea::horizontal()
        .auto_shrink([false, false]) // No automatic shrinkage
        .show(ui, |ui| {
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(name_max_width)) // Name
                .column(Column::initial(100.0)); // Avg Std

            for w in &file_widths {
                table = table
                    .column(Column::initial(*w)) // Std
                    .column(Column::initial(*w)); // Raw
            }

            let mut sorted_people: Vec<PersonScore> = state
                .all_people
                .iter()
                .map(|name: &String| {
                    let mut std_sum = 0.0f32;
                    let mut std_cnt = 0usize;
                    let mut scores: Vec<Option<(f32, f32)>> = Vec::new();

                    for file in &state.file_order {
                        if let Some((s, raw)) = compute_std_raw_for(&state.per_file, file, name) {
                            scores.push(Some((s, raw)));
                            std_sum += s;
                            std_cnt += 1;
                        } else {
                            scores.push(None);
                        }
                    }

                    let avg_std = if std_cnt > 0 {
                        std_sum / (std_cnt as f32)
                    } else {
                        0.0
                    };

                    PersonScore {
                        name: name.clone(),
                        avg_std,
                        scores,
                    }
                })
                .collect();

            sorted_people.sort_by(|a, b| b.avg_std.partial_cmp(&a.avg_std).unwrap());

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Name");
                    });
                    header.col(|ui| {
                        ui.strong("Avg Std");
                    });
                    for file in &state.file_order {
                        header.col(|ui| {
                            ui.strong(format!("{} Std", file));
                        });
                        header.col(|ui| {
                            ui.strong(format!("{} Raw", file));
                        });
                    }
                })
                .body(|mut body| {
                    for PersonScore {
                        name,
                        avg_std,
                        scores,
                    } in &sorted_people
                    {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(name);
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.*}", state.precision, avg_std));
                            });

                            for score in scores {
                                if let Some((std, raw)) = score {
                                    row.col(|ui| {
                                        ui.label(format!("{:.*}", state.precision, std));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:.*}", state.precision, raw));
                                    });
                                } else {
                                    row.col(|ui| {
                                        ui.label("-");
                                    });
                                    row.col(|ui| {
                                        ui.label("-");
                                    });
                                }
                            }
                        });
                    }
                });
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
    let std_score = (raw / fr.highest_non_std) * 100.0;

    Some((std_score, raw))
}

fn setup_chinese_fonts(ctx: &egui::Context) {
    // Infer font directory from SystemRoot on Windows only; other platforms can extend it
    let system_root = env::var("SystemRoot").unwrap_or_else(|_| "/Windows".to_string());

    // Try to load Noto Sans SC
    let noto_path = PathBuf::from(format!("{system_root}/Fonts/NotoSansSC-Regular.ttf"));
    println!("Noto Sans SC path: {}", noto_path.display());

    let font_data = if noto_path.exists() {
        println!("Use Noto Sans SC font");
        fs::read(noto_path).ok()
    } else {
        // Fallback to Microsoft YaHei
        println!("Noto Sans SC does not exist, fallback to system fonts: Microsoft YaHei");
        let msyh_path = format!("{system_root}/Fonts/msyh.ttc");
        fs::read(msyh_path).ok()
    };

    let mut fonts = FontDefinitions::default();
    if let Some(data) = font_data {
        // Key: Convert FontData to Arc<FontData>
        fonts
            .font_data
            .insert("chinese_font".to_owned(), FontData::from_owned(data).into());

        // Put Chinese fonts on top of the proportional and monospace families, rendering Chinese first
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "chinese_font".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "chinese_font".to_owned());

        ctx.set_fonts(fonts);
    } else {
        eprintln!("Failed to load any Chinese fonts, please check the font path");
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "std score calculator",
        options,
        Box::new(|cc| {
            setup_chinese_fonts(&cc.egui_ctx);
            // Key: Return Result<Box<dyn App>, _>
            Ok(Box::new(StdScoreApp::default()))
        }),
    )
}
