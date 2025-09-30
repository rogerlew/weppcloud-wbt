use crate::MyApp;
use egui::{CollapsingHeader, ScrollArea};

impl MyApp {
    pub fn tools_panel(&mut self, ctx: &egui::Context) {
        // Tool tree side panel
        egui::SidePanel::left("tool_panel").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.small(" "); // just to put some vertical space between the header and the top.
                ui.heading(&format!("🛠 {} Available Tools", self.num_tools));
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui.toggle_value(&mut self.state.show_toolboxes, "Toolboxes")
                .on_hover_text("Search for tools in their toolboxes")
                .clicked() {
                    self.state.show_toolboxes = true;
                    self.state.show_tool_search = false;
                    self.state.show_recent_tools = false;
                }
                if ui.toggle_value(&mut self.state.show_tool_search, "Tool Search")
                .on_hover_text("Search for tools by keywords")
                .clicked() {
                    self.state.show_toolboxes = false;
                    self.state.show_tool_search = true;
                    self.state.show_recent_tools = false;
                }
                if ui.toggle_value(&mut self.state.show_recent_tools, "Recent Tools")
                .on_hover_text("List recently used and most used tools.")
                .clicked() {
                    self.state.show_toolboxes = false;
                    self.state.show_tool_search = false;
                    self.state.show_recent_tools = true;
                }
                // ui.label("          "); // to make the panel wide enough for some of the longer names.
            });
            ui.separator();
                    
            let mut clicked_tool = String::new();
            ui.vertical(|ui| {
                if self.state.show_toolboxes {
                    // ui.vertical_centered(|ui| {
                    //     ui.label(&format!("🛠 {} Available Tools", self.num_tools));
                    // });
                    ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // let mut clicked_tool = String::new();
                        // self.tree.ui(ui); // This is a recursive approach that works better, but can't access MyApp.

                        // What follows is truly awful and fragile code. It relies on the fact that
                        // there are only 1-level sub-folders and no 2-level sub-folders. Should this
                        // ever change in the future, this would need to be updated.
                        // CollapsingHeader::new(&self.tree.label)
                        // .default_open(&self.tree.label == "Toolboxes")
                        // // .icon(circle_icon)
                        // .show(ui, |ui| {
                            // render the toolboxes
                            for i in 0..self.tree.children.len() {

                                let tree = &self.tree.children[i];
                                CollapsingHeader::new(
                                    egui::RichText::new(&tree.label)
                                    .strong()
                                    // .italics()
                                    // .color(ui.visuals().hyperlink_color)
                                    // .background_color(ui.visuals().selection.bg_fill)
                                    // .color(ui.visuals().selection.stroke.color)
                                )
                                .default_open(false)
                                // .icon(circle_icon)
                                .show(ui, |ui| {
                                    for j in 0..tree.children.len() {
                                        let tree2 = &tree.children[j];
                                        if tree2.is_toolbox() {
                                            CollapsingHeader::new(
                                                egui::RichText::new(&tree2.label)
                                                .strong()
                                                // .italics()
                                                // .color(ui.visuals().hyperlink_color)
                                                // .background_color(ui.visuals().selection.bg_fill)
                                                // .color(ui.visuals().selection.stroke.color)
                                            )
                                            .default_open(false)
                                            // .icon(circle_icon)
                                            .show(ui, |ui| {
                                                for k in 0..tree2.children.len() {
                                                    let tree3 = &tree2.children[k];
                                                    let tool_index = *self.tool_order.get(&tree3.label.clone()).unwrap();

                                                    // if ui.toggle_value(&mut self.open_tools[tool_index], &format!("🔧 {}", tree3.label))
                                                    
                                                    if ui.button(&format!("🔧 {}", tree3.label))
                                                    .on_hover_text(self.tool_descriptions.get(&tree3.label).unwrap_or(&String::new()))
                                                    .clicked() {
                                                        clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                                    }

                                                    // if ui.add(egui::Button::new(&format!("🔧 {}", tree3.label)).fill(egui::Color32::from_rgb(224, 240, 255))
                                                    // ).on_hover_text(self.tool_descriptions.get(&tree3.label).unwrap_or(&String::new())).clicked() {
                                                    //     clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                                    // }
                                                }
                                            });
                                        } else { // it's a tool
                                            let tool_index = *self.tool_order.get(&tree2.label.clone()).unwrap();
                                            // if ui.toggle_value(&mut self.open_tools[tool_index], &format!("🔧 {}", tree2.label))
                                            if ui.button(&format!("🔧 {}", tree2.label))
                                            .on_hover_text(self.tool_descriptions.get(&tree2.label).unwrap_or(&String::new()))
                                            .clicked() {
                                                // self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                                clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                            }

                                            // if ui.add(egui::Button::new(&format!("🔧 {}", tree2.label)).fill(egui::Color32::from_rgb(224, 240, 255))
                                            // ).on_hover_text(self.tool_descriptions.get(&tree2.label).unwrap_or(&String::new()))
                                            // .clicked() {
                                            //     clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                            // }
                                        }
                                    }
                                });
                            }
                        // });

                        // let margin = ui.visuals().clip_rect_margin;
                        // let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                        // let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                        // (current_scroll, max_scroll)
                    })
                    .inner;

                } else if self.state.show_tool_search {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // ui.label("Keywords:")
                            // .on_hover_text("Search for keywords (separated by commas) in tool names or descriptions");
                            ui.label(
                                egui::RichText::new("Keywords:")
                                // .italics()
                                .strong()
                                // .color(ui.visuals().hyperlink_color)
                            )
                            .on_hover_text("Search for keywords in tool names or descriptions. Keywords should be separated by spaces (AND) or commas (OR). AND (&) and OR (|) operators are also valid to combine search words.");

                            // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            //     if ui.button("Clear").on_hover_text("Clear search keywords").clicked() {
                            //         self.search_words_str = "".to_string();
                            //     }
                            // });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search_words_str)
                                .desired_width(self.state.textbox_width)
                                
                                // .on_hover_text("Search for keywords (separated by commas) in tool names or descriptions");
                            );

                            if ui.button("✖").on_hover_text("Clear search keywords").clicked() {
                                self.search_words_str = "".to_string();
                            }
                        });

                        ui.small(""); // just a vertical spacer

                        ui.horizontal(|ui| {
                            if self.num_search_hits != 1 {
                                ui.label(&format!("Found {} tools", self.num_search_hits));
                            } else {
                                ui.label(&format!("Found {} tool", self.num_search_hits));
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.checkbox(&mut self.case_sensitive_search, "Case sensitive");
                            });
                        });

                        ui.separator();
                        
                        if !self.search_words_str.trim().is_empty() {
                            ScrollArea::vertical()
                            .max_height(f32::INFINITY)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                // Perform the search...
                                let mut found: bool;
                                let search_words_str = self.search_words_str
                                                        .replace("||", ",").replace("|", ",")
                                                        .replace(" OR ", ",").replace(" or ", ",")
                                                        .replace(" AND ", "&").replace(" and ", "&")
                                                        .replace(" & ", "&").replace(" ", "&");
                                let search_words = search_words_str.split(",").collect::<Vec<&str>>();
                                let mut hs = std::collections::HashSet::new();
                                for k in 0..search_words.len() {
                                    let mut sw_raw = search_words[k].trim().replace("AND", "&");
                                    if !self.case_sensitive_search {
                                        sw_raw = sw_raw.to_lowercase();
                                    }
                                    let sw_list = sw_raw.split("&").collect::<Vec<&str>>();
                                    for tool_info in &self.tool_info {
                                        let mut tn = tool_info.tool_name.to_string();
                                        let mut desc = self.tool_descriptions.get(&tn).unwrap_or(&String::new()).clone();
                                        if !self.case_sensitive_search {
                                            tn = tn.to_lowercase();
                                        }
                                        if !self.case_sensitive_search {
                                            desc = desc.to_lowercase();
                                        }
                                        found = true;
                                        for sw in &sw_list {
                                            // if !self.case_sensitive_search {
                                            //     if tn.contains(sw) {
                                            //         println!("{} {} {} {}", tn, sw, tn.contains(sw), sw_list.len());
                                            //     }
                                            // }
                                            if !tn.contains(sw) && !desc.contains(sw) {
                                                // At least one of the compound search words is not 
                                                // in this tool name/description.
                                                found = false;
                                                break;
                                            }
                                        }
                                        if found { hs.insert(tool_info.tool_name.to_string()); }
                                    }
                                }

                                self.num_search_hits = hs.len();

                                if !hs.is_empty() {
                                    let mut tools: Vec<_> = hs.into_iter().collect();
                                    tools.sort();

                                    for tool in tools {
                                        // ui.label(format!("{}", tool));
                                        if let Some(tool_index) = self.tool_order.get(&tool) {
                                            // if ui.toggle_value(&mut self.open_tools[tool_index], &tool)
                                            // .on_hover_text(self.tool_descriptions.get(&tool).unwrap_or(&String::new()))
                                            // .clicked() {
                                            //     self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                            //     // let tn = self.tool_info[tool_index].tool_name.clone();
                                            //     // self.update_recent_tools(&tn);
                                            //     clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                            // }
                                            if ui.button(&tool)
                                            .on_hover_text(self.tool_descriptions.get(&tool).unwrap_or(&String::new()))
                                            .clicked() {
                                                // self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                                clicked_tool = self.tool_info[*tool_index].tool_name.clone();
                                            }
                                        }
                                    }
                                }

                                // let margin = ui.visuals().clip_rect_margin;

                                // let current_scroll2 = ui.clip_rect().top() - ui.min_rect().top() + margin;
                                // let max_scroll2 = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                                // (current_scroll2, max_scroll2)
                            })
                            .inner;
                        } else if self.num_search_hits > 0 {
                            self.num_search_hits = 0;
                        }

                    });
                } else if self.state.show_recent_tools {
                    ui.vertical(|ui| {
                        ScrollArea::vertical()
                        .id_source("recently_used_tools")
                        .max_height(f32::INFINITY)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // ui.label("Recently used tools:");
                                ui.label(
                                    egui::RichText::new("Recently used tools:")
                                    // .italics()
                                    .strong()
                                    // .color(ui.visuals().hyperlink_color)
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🔃").on_hover_text("Reset recent tools").clicked() {
                                        self.state.most_recent.clear();
                                    }
                                });
                            });

                            for tool in &self.state.most_recent {
                                // ui.label(format!("{}", tool));
                                let tool_index = *self.tool_order.get(tool).unwrap();
                                // if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                                // .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                                // .clicked() {
                                //     self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                //     // let tn = self.tool_info[tool_index].tool_name.clone();
                                //     // self.update_recent_tools(&tn);
                                //     // clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                // }
                                if ui.button(tool)
                                .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                                .clicked() {
                                    // self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                    clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                }
                            }

                            ui.separator();
                            ui.horizontal(|ui| {
                                // ui.label("Most-used tools:");
                                ui.label(
                                    egui::RichText::new("Most-used tools:")
                                    // .italics()
                                    .strong()
                                    // .color(ui.visuals().hyperlink_color)
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🔃").on_hover_text("Reset most-used tools").clicked() {
                                        self.most_used.clear();
                                        self.most_used_hm.clear();
                                    }
                                });
                            });

                            for val in &self.most_used {
                                let tool_index = *self.tool_order.get(&val.1).unwrap();
                                // if ui.toggle_value(&mut self.open_tools[tool_index], &format!("{} ({})", val.1, val.0))
                                // .on_hover_text(self.tool_descriptions.get(&val.1).unwrap_or(&String::new()))
                                // .clicked() {
                                //     self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                // }
                                if ui.button(&format!("{} ({})", val.1, val.0))
                                .on_hover_text(self.tool_descriptions.get(&val.1).unwrap_or(&String::new()))
                                .clicked() {
                                    // self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                    clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                }
                            }
                        });
                    }).inner;

                    // ui.vertical(|ui| {
                    //     ui.horizontal(|ui| {
                    //         ui.label("Recently used tools:");
                    //         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //             if ui.button("Clear").on_hover_text("Clear recent tools").clicked() {
                    //                 self.state.most_recent.clear();
                    //             }
                    //         });
                    //     });

                    //     ScrollArea::vertical()
                    //     .id_source("recently_used_tools")
                    //     .max_height(300.0) //f32::INFINITY)
                    //     .auto_shrink([false; 2])
                    //     .show(ui, |ui| {
                    //         for tool in &self.state.most_recent {
                    //             // ui.label(format!("{}", tool));
                    //             let tool_index = *self.tool_order.get(tool).unwrap();
                    //             if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                    //             .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                    //             .clicked() {
                    //                 self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                    //             }
                    //         }
                    //     })
                    //     .inner;

                    //     ui.separator();
                    //     ui.horizontal(|ui| {
                    //         ui.label("Most used tools:");
                    //         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //             if ui.button("Clear").on_hover_text("Clear recent tools").clicked() {
                    //                 self.state.most_recent.clear();
                    //             }
                    //         });
                    //     });

                    //     ScrollArea::vertical()
                    //     .id_source("most_used_tools")
                    //     .max_height(300.0) //f32::INFINITY)
                    //     .auto_shrink([false; 2])
                    //     .show(ui, |ui| {
                    //         ui.label("Hello");
                    //     //     for tool in &self.state.most_recent {
                    //     //         // ui.label(format!("{}", tool));
                    //     //         let tool_index = *self.tool_order.get(tool).unwrap();
                    //     //         if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                    //     //         .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                    //     //         .clicked() {
                    //     //             self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                    //     //         }
                    //     //     }
                    //     })
                    //     .inner;
                    // });
                }
            });

            if !clicked_tool.is_empty() {
                self.update_recent_tools(&clicked_tool);
            }
            
        });
    }
}
