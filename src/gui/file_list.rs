use crate::{
    config, locale,
    logic::{self, AssetInfo},
};
use crate::gui::image_preview;
// Used for functionality
use fluent_bundle::{FluentBundle, FluentResource};
use native_dialog::{DialogBuilder, MessageLevel};
use std::sync::Arc;

fn double_click(
    asset: logic::AssetInfo,
    swapping: &mut bool,
    copying: &mut bool,
    swapping_asset: &mut Option<logic::AssetInfo>,
) {
    if *copying {
        if swapping_asset.is_none() {
            *swapping_asset = Some(asset);
        } else if let Some(ref src_asset) = *swapping_asset {
            logic::copy_assets(src_asset.clone(), asset);
            *swapping_asset = None;
            *copying = false;
        }
    } else if *swapping {
        if swapping_asset.is_none() {
            *swapping_asset = Some(asset);
        } else if let Some(ref src_asset) = *swapping_asset {
            logic::swap_assets(src_asset.clone(), asset);
            *swapping_asset = None;
            *swapping = false
        }
    } else {
        let temp_dir = logic::get_temp_dir();
        let alias = config::get_asset_alias(&asset.name);
        let destination = temp_dir.join(alias);
        match logic::extract_to_file(asset, destination.clone(), true) {
            Ok(new_destination) => match open::that(new_destination) {
                Ok(()) => (),
                Err(err) => {
                    logic::update_status(locale::get_message(
                        &locale::get_locale(None),
                        "failed-opening-file",
                        None,
                    ));
                    log_error!("Failed opening file: {}", err)
                }
            },
            Err(e) => {
                logic::update_status(locale::get_message(
                    &locale::get_locale(None),
                    "failed-opening-file",
                    None,
                ));
                log_error!("Failed opening file: {}", e)
            }
        }
    }
}

fn extract_all_of_type(category: logic::Category, locale: &FluentBundle<Arc<FluentResource>>) {
    let mut no = logic::get_list_task_running();

    // Confirmation dialog, the program is still listing files
    if no {
        // NOT result, will become false if user clicks yes
        no = !DialogBuilder::message()
            .set_level(MessageLevel::Info)
            .set_title(locale::get_message(
                locale,
                "confirmation-filter-confirmation-title",
                None,
            ))
            .set_text(locale::get_message(
                locale,
                "confirmation-filter-confirmation-description",
                None,
            ))
            .confirm()
            .show()
            .unwrap_or(false);
    }

    // The user either agreed or the program is not listing files
    if !no {
        // If the user provides a directory, the program will extract the assets to that directory
        if let Ok(Some(path)) = DialogBuilder::file().open_single_dir().show() {
            logic::extract_dir(
                path,
                category,
                false,
                config::get_config_bool("use_alias").unwrap_or(false),
            );
        }
    }
}
fn toggle_swap(
    swapping: &mut bool,
    swapping_asset: &mut Option<AssetInfo>,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    let mut warning_acknowledged = config::get_config_bool("ban-warning-ack").unwrap_or(false);

    if !warning_acknowledged {
        warning_acknowledged = DialogBuilder::message()
            .set_level(MessageLevel::Info)
            .set_title(locale::get_message(
                locale,
                "confirmation-ban-warning-title",
                None,
            ))
            .set_text(locale::get_message(
                locale,
                "confirmation-ban-warning-description",
                None,
            ))
            .confirm()
            .show()
            .unwrap();
    }

    if warning_acknowledged {
        config::set_config_value("ban-warning-ack", warning_acknowledged.into());
        if *swapping {
            *swapping_asset = None;
        }
        *swapping = !*swapping;
    }
}

fn extract_file_button(asset: logic::AssetInfo) {
    let alias = config::get_asset_alias(&asset.name);
    if let Some(destination) = native_dialog::DialogBuilder::file()
        .set_filename(&alias)
        .save_single_file()
        .show()
        .unwrap()
    {
        match logic::extract_to_file(asset, destination, false) {
            Ok(_) => (),
            Err(e) => log_critical!("{}", e),
        }
    }
}

fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>, ctx: &egui::Context) {
    // Confirmation dialog
    let yes = DialogBuilder::message()
        .set_level(MessageLevel::Info)
        .set_title(locale::get_message(
            locale,
            "confirmation-clear-cache-title",
            None,
        ))
        .set_text(locale::get_message(
            locale,
            "confirmation-clear-cache-description",
            None,
        ))
        .confirm()
        .show()
        .unwrap();

    if yes {
        image_preview::clear_all_images(ctx);
        logic::clear_cache();
    }
}

fn toggle_swap_or_copy(
    swapping_or_copying: &mut bool,
    swapping_asset: &mut Option<AssetInfo>,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    let mut warning_acknowledged = config::get_config_bool("ban-warning-ack").unwrap_or(false);

    if !warning_acknowledged {
        warning_acknowledged = DialogBuilder::message()
            .set_level(MessageLevel::Info)
            .set_title(locale::get_message(
                locale,
                "confirmation-ban-warning-title",
                None,
            ))
            .set_text(locale::get_message(
                locale,
                "confirmation-ban-warning-description",
                None,
            ))
            .confirm()
            .show()
            .unwrap();
    }

    if warning_acknowledged {
        config::set_config_value("ban-warning-ack", warning_acknowledged.into());
        if *swapping_or_copying {
            *swapping_asset = None;
        }
        *swapping_or_copying = !*swapping_or_copying;
    }
}

pub struct FileListUi {
    selected: Option<usize>,
    current_tab: Option<String>,
    renaming: bool,
    searching: bool,
    search_query: String,
    swapping: bool,
    swapping_asset: Option<logic::AssetInfo>,
    asset_context_menu_open: Option<usize>,
    copying: bool,
    pub locale: FluentBundle<Arc<FluentResource>>,
}

impl FileListUi {
    fn handle_text_edit(&mut self, ui: &mut egui::Ui, alias: &str, file_name: &str) {
        let mut mutable_name = alias.to_string();
        let response = egui::TextEdit::singleline(&mut mutable_name)
            .hint_text(file_name)
            .show(ui)
            .response;

        if mutable_name != alias {
            config::set_asset_alias(file_name, &mutable_name);
        }

        if response.lost_focus() {
            self.renaming = false;
            if mutable_name.is_empty() {
                config::set_asset_alias(file_name, file_name); // Set it to file name if blank
            }
        } else {
            response.request_focus(); // Request focus if it hasn't lost focus
        }
    }

    fn asset_buttons(
        &mut self,
        ui: &mut egui::Ui,
        category: logic::Category,
        focus_search_box: &mut bool,
        asset: Option<AssetInfo>,
    ) {
        if let Some(asset) = asset.clone() {
            if ui
                .button(locale::get_message(&self.locale, "button-open", None))
                .clicked()
            {
                double_click(
                    asset.clone(),
                    &mut self.swapping,
                    &mut self.copying,
                    &mut self.swapping_asset,
                );
                self.asset_context_menu_open = None;
            }
            if ui
                .button(locale::get_message(
                    &self.locale,
                    "button-extract-file",
                    None,
                ))
                .clicked()
            {
                extract_file_button(asset);
                self.asset_context_menu_open = None;
            }
        }
        if ui
            .button(locale::get_message(&self.locale, "button-search", None))
            .clicked()
        {
            self.searching = !self.searching;
            *focus_search_box = true;
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(&self.locale, "button-rename", None))
            .clicked()
        {
            // Rename button
            self.renaming = !self.renaming;
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(
                &self.locale,
                "button-clear-cache",
                None,
            ))
            .clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Delete))
        {
            clear_cache(&self.locale, ui.ctx());
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(
                &self.locale,
                "button-extract-type",
                None,
            ))
            .clicked()
        {
            extract_all_of_type(category, &self.locale);
            self.asset_context_menu_open = None;
        }
        if ui
            .button(locale::get_message(&self.locale, "button-refresh", None))
            .clicked()
        {
            logic::refresh(category, false, false);
            self.asset_context_menu_open = None;
        }
        if ui
            .button(locale::get_message(&self.locale, "button-swap", None))
            .clicked()
        {
            toggle_swap(&mut self.swapping, &mut self.swapping_asset, &self.locale);
            self.asset_context_menu_open = None;

            if let Some(n) = asset.clone() {
                self.swapping_asset = Some(n);
            } else {
                self.swapping_asset = None;
            }
        }
        if ui
            .button(locale::get_message(&self.locale, "button-copy", None))
            .clicked()
        {
            toggle_swap_or_copy(&mut self.copying, &mut self.swapping_asset, &self.locale);
            self.asset_context_menu_open = None;

            if let Some(n) = asset.clone() {
                self.swapping_asset = Some(n);
            } else {
                self.swapping_asset = None;
            }
        }

        if category == logic::Category::Images || category == logic::Category::Ktx {
            let message = if config::get_config_bool("display_image_preview").unwrap_or(false) {
                locale::get_message(&self.locale, "button-disable-display-image-preview", None)
            } else {
                locale::get_message(&self.locale, "button-display-image-preview", None)
            };

            if ui.button(message).clicked() {
                config::set_config_value(
                    "display_image_preview",
                    (!config::get_config_bool("display_image_preview").unwrap_or(false)).into(),
                );
                self.asset_context_menu_open = None;
            }
        }
    }

    // Function to handle asset response within asset list
    fn handle_asset_response(
        &mut self,
        response: egui::Response,
        visuals: &egui::Visuals,
        is_selected: bool,
        i: usize,
        scroll_to: Option<usize>,
        navigation_accepted: &mut bool,
        focus_search_box: &mut bool,
        asset: AssetInfo,
    ) -> (egui::Color32, egui::Color32) {
        // Highlight the background when selected
        let background_colour = if is_selected {
            visuals.selection.bg_fill // Primary colour
        } else {
            egui::Color32::TRANSPARENT // No background colour
        };

        // Make the text have more contrast when selected
        let text_colour = if is_selected {
            visuals.strong_text_color() // Brighter
        } else {
            visuals.text_color() // Normal
        };

        // Handle the click/double click
        if response.clicked() && !self.renaming {
            self.selected = Some(i);
        }

        if response.secondary_clicked() {
            self.selected = Some(i);
            self.asset_context_menu_open = Some(i);
        }

        if let Some(asset_context_menu_open) = self.asset_context_menu_open {
            if asset_context_menu_open == i {
                response.context_menu(|ui| {
                    self.asset_buttons(ui, asset.category, focus_search_box, Some(asset.clone()));
                });
            }
        }

        if response.double_clicked() {
            double_click(
                asset,
                &mut self.swapping,
                &mut self.copying,
                &mut self.swapping_asset,
            );
        }

        // Handle keyboard scrolling
        if scroll_to == Some(i) {
            *navigation_accepted = true;
            response.scroll_to_me(Some(egui::Align::Center)) // Align to center to prevent scrolling off the edge
        }

        (background_colour, text_colour)
    }

    pub fn ui(&mut self, tab: String, ui: &mut egui::Ui) {
        let category = match tab.as_str() {
            "music" => logic::Category::Music,
            "sounds" => logic::Category::Sounds,
            "images" => logic::Category::Images,
            "ktx-files" => logic::Category::Ktx,
            "rbxm-files" => logic::Category::Rbxm,
            _ => logic::Category::All,
        };

        // Detect if tab changed and do a refresh if so
        if let Some(current_tab) = &self.current_tab {
            if current_tab != &tab {
                self.current_tab = Some(tab.to_owned());
                logic::refresh(category, false, false);
            }
        } else {
            self.current_tab = Some(tab.to_owned());
            logic::refresh(category, false, false);
        }

        let file_list = logic::get_file_list();

        let mut focus_search_box = false; // Focus the search box toggle for this frame

        // Apply current sort to the file list
        let file_list = if self.searching {
            let old_search_query = self.search_query.clone();

            let response = ui.text_edit_singleline(&mut self.search_query);

            if focus_search_box {
                response.request_focus();
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.searching = false; // Remove the search bar when the use presses escape
            }

            if self.search_query != old_search_query {
                logic::filter_file_list(self.search_query.clone());
            }
            let mut list = logic::get_filtered_file_list();
            logic::apply_sort(&mut list);
            list
        } else {
            let mut list = file_list;
            logic::apply_sort(&mut list);
            list
        };

        // Handle F5 refresh shortcut before empty check so it works even when no assets
        if ui.input(|i| i.key_pressed(egui::Key::F5)) {
            logic::refresh(category, false, false);
        }

        // Empty state
        if file_list.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                let icon_size = 48.0;
                ui.add(
                    egui::Label::new(egui::RichText::new("📂").size(icon_size)).selectable(false),
                );
                ui.heading(locale::get_message(&self.locale, "empty-state-title", None));
                ui.label(locale::get_message(
                    &self.locale,
                    "empty-state-description",
                    None,
                ));
                ui.label(locale::get_message(&self.locale, "empty-state-hint", None));
                ui.add_space(20.0);
                if ui
                    .button(locale::get_message(&self.locale, "button-refresh", None))
                    .clicked()
                {
                    logic::refresh(category, false, false);
                }
            });
            return;
        }

        // Handle key shortcuts here
        if ui.input(|i| i.key_pressed(egui::Key::F2)) {
            // Rename hotkey
            self.renaming = !self.renaming;
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
            // Ctrl+F (Search)
            self.searching = !self.searching;
            focus_search_box = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !self.renaming {
            // del key used for editing, don't allow during editing
            clear_cache(&self.locale, ui.ctx());
        }
        if ui.input(|i| i.key_pressed(egui::Key::F3)) {
            extract_all_of_type(category, &self.locale);
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            // Ctrl+D (Swap)
            toggle_swap_or_copy(&mut self.swapping, &mut self.swapping_asset, &self.locale);
            if let Some(i) = self.selected {
                self.swapping_asset = file_list.get(i).cloned();
            } else {
                self.swapping_asset = None;
            }
        }
        if ui.input(|inp| inp.events.iter().any(|ev| matches!(ev, egui::Event::Copy))) {
            // https://github.com/emilk/egui/issues/4065#issuecomment-2071047410
            // Ctrl+C (Copy)
            toggle_swap_or_copy(&mut self.copying, &mut self.swapping_asset, &self.locale);
            if let Some(i) = self.selected {
                self.swapping_asset = file_list.get(i).cloned();
            } else {
                self.swapping_asset = None;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !self.searching {
            // Esc (Cancel actions)
            self.swapping_asset = None;
            self.copying = false;
            self.swapping = false;
        }

        // GUI logic below here

        // Top UI buttons
        if config::get_config_bool("use_topbar_buttons").unwrap_or(true) {
            ui.push_id("Topbar buttons", |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.asset_buttons(ui, category, &mut focus_search_box, None);
                    });
                })
            });
        }

        let mut scroll_to: Option<usize> = None; // This is reset every frame, so it doesn't constantly scroll to the same label
        let mut none_selected: bool = false; // Used to scroll to the first value shown when none is selected

        // Only allow navigation of the user is not renaming
        if !self.renaming {
            // If the user presses up, decrement the selected value
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if let Some(selected) = self.selected {
                    if selected > 0 {
                        // Check if it is larger than 0 otherwise it'll attempt to select non-existant labels
                        self.selected = Some(selected - 1);
                        scroll_to = Some(selected - 1); // This is also set to the same number, allowing for auto scrolling
                    }
                } else {
                    none_selected = true // Select the first visible entry
                }
            }

            // If the user presses down, increment the selected value
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                if let Some(selected) = self.selected {
                    if selected < file_list.len() - 1 {
                        // Stop it from overflowing otherwise it'll attempt to select non-existant labels
                        self.selected = Some(selected + 1);
                        scroll_to = Some(selected + 1); // This is also set to the same number, allowing for auto scrolling
                    }
                } else {
                    none_selected = true // Select the first visible entry
                }
            }

            // Allow the user to confirm with enter
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(selected) = self.selected {
                    // Get file name after getting the selected value
                    if let Some(asset) = file_list.get(selected) {
                        double_click(
                            asset.clone(),
                            &mut self.swapping,
                            &mut self.copying,
                            &mut self.swapping_asset,
                        );
                    }
                }
            }

            if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::E)) {
                // Ctrl+E (Extract)
                if let Some(selected) = self.selected {
                    // Get file name after getting the selected value
                    if let Some(asset) = file_list.get(selected) {
                        extract_file_button(asset.clone());
                    }
                }
            }
        }

        let mut navigation_accepted: bool = false; // Used to check if the selected label is available to accept the keyboard navigation

        if self.swapping {
            if let Some(ref asset) = self.swapping_asset {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set(
                    "asset",
                    config::get_asset_alias(&asset.name),
                );
                ui.heading(locale::get_message(&self.locale, "swap-with", Some(&args)));
            } else {
                ui.heading(locale::get_message(&self.locale, "swap-choose-file", None));
            }
        }

        if self.copying {
            if let Some(ref asset) = self.swapping_asset {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set(
                    "asset",
                    config::get_asset_alias(&asset.name),
                );
                ui.heading(locale::get_message(
                    &self.locale,
                    "overwrite-with",
                    Some(&args),
                ));
            }
        }

        let is_preview_tab = tab == "images" || tab == "ktx-files";
        let display_image_preview =
            config::get_config_bool("display_image_preview").unwrap_or(false) && is_preview_tab;

        let row_height = if display_image_preview {
            config::get_config_u64("image_preview_size").unwrap_or(128) as f32
        } else {
            ui.text_style_height(&egui::TextStyle::Body)
        };

        let amount_per_row = if display_image_preview {
            ui.available_width() as usize / (row_height + 7.5) as usize // Account for padding because ui.horizontal adds padding
        } else {
            1
        };

        let total_rows = if display_image_preview {
            f32::ceil(file_list.len() as f32 / amount_per_row as f32) as usize
        // Show even unfilled rows
        } else {
            file_list.len()
        };

        // Column headers with sorting (only in list view)
        if !display_image_preview {
            let full_width = ui.available_width();
            let header_height = ui.text_style_height(&egui::TextStyle::Small);
            let (header_rect, header_response) =
                ui.allocate_exact_size(egui::vec2(full_width, header_height), egui::Sense::click());

            let visuals = ui.visuals().clone();
            let header_bg = visuals.widgets.noninteractive.bg_fill;
            ui.painter().rect_filled(header_rect, 0.0, header_bg);

            let sort_col = logic::get_sort_column();
            let sort_dir = logic::get_sort_direction();
            let sort_indicator = match (&sort_col, &sort_dir) {
                (logic::SortColumn::Name, logic::SortDirection::Ascending) => " ▲",
                (logic::SortColumn::Name, logic::SortDirection::Descending) => " ▼",
                _ => "",
            };

            let name_label = format!(
                "{}{}",
                locale::get_message(&self.locale, "col-name", None),
                sort_indicator
            );
            let name_text = egui::RichText::new(&name_label)
                .text_style(egui::TextStyle::Small)
                .color(visuals.text_color());

            ui.put(
                egui::Rect::from_min_size(
                    header_rect.min + egui::vec2(8.0, 0.0),
                    egui::vec2(header_rect.width() * 0.7, header_height),
                ),
                egui::Label::new(name_text).selectable(false),
            );

            let source_label = "Source";
            let source_text = egui::RichText::new(source_label)
                .text_style(egui::TextStyle::Small)
                .color(visuals.text_color());

            ui.put(
                egui::Rect::from_min_size(
                    header_rect.min + egui::vec2(header_rect.width() * 0.7, 0.0),
                    egui::vec2(header_rect.width() * 0.3, header_height),
                ),
                egui::Label::new(source_text).selectable(false),
            );

            if header_response.clicked() {
                logic::toggle_sort(logic::SortColumn::Name);
                let mut list = if self.searching {
                    logic::get_filtered_file_list()
                } else {
                    logic::get_file_list()
                };
                logic::apply_sort(&mut list);
            }

            ui.separator();
        }

        // File list for assets
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            row_height,
            total_rows,
            |ui, row_range| {
                if display_image_preview {
                    for row_idx in row_range {
                        ui.horizontal(|ui| {
                            for amount in 0..amount_per_row {
                                let i = (row_idx * amount_per_row) + amount;
                                if let Some(asset) = file_list.get(i) {
                                    let file_name = &asset.name;
                                    let alias = config::get_asset_alias(file_name);

                                    let is_selected = if none_selected && i != 0 {
                                        // Selecting the very first causes some issues
                                        self.selected = Some(i); // If there is none selected, Set selected and return true
                                        none_selected = false; // Will select everything if this is not set to false immediately
                                        true
                                    } else {
                                        self.selected == Some(i) // Check if this current one is selected
                                    };

                                    // Draw the text
                                    if is_selected && self.renaming {
                                        self.handle_text_edit(ui, &alias, file_name);
                                    // Allow user to edit
                                    } else {
                                        let desired_size = egui::vec2(row_height, row_height); // Set height to the text style height
                                        let (rect, response) = ui.allocate_exact_size(
                                            desired_size,
                                            egui::Sense::click(),
                                        );

                                        // Only attempt to load if it's a real asset
                                        if asset.from_file | asset.from_sql | asset.from_rbx_storage
                                        {
                                            if let Some(texture) =
                                                image_preview::load_asset_image(asset.clone(), ui.ctx().clone())
                                            {
                                                egui::Image::new(&texture)
                                                    .maintain_aspect_ratio(true)
                                                    .max_height(row_height)
                                                    .paint_at(ui, rect);
                                            }
                                        }

                                        let visuals = ui.visuals();

                                        // Get colours and handle response
                                        let colours = self.handle_asset_response(
                                            response,
                                            visuals,
                                            is_selected,
                                            i,
                                            scroll_to,
                                            &mut navigation_accepted,
                                            &mut focus_search_box,
                                            asset.clone(),
                                        );

                                        let text_colour = colours.1;
                                        let background_colour = colours.0;

                                        // Draw the background colour
                                        ui.painter().rect_stroke(
                                            rect,
                                            0.0,
                                            egui::Stroke::new(row_height / 8.0, background_colour),
                                            egui::StrokeKind::Inside,
                                        );

                                        // Draw text ontop of image
                                        let text = egui::Label::new(
                                            egui::RichText::new(alias)
                                                .text_style(egui::TextStyle::Body)
                                                .color(text_colour),
                                        )
                                        .truncate()
                                        .selectable(false);

                                        let text_size =
                                            ui.text_style_height(&egui::TextStyle::Body);

                                        let text_rect = egui::Rect::from_min_size(
                                            rect.min
                                                + egui::vec2(
                                                    0.0,
                                                    (rect.height() - text_size) / 2.0,
                                                ),
                                            egui::vec2(row_height, text_size),
                                        );

                                        // Background to make text easier to read
                                        let background_colour = if visuals.dark_mode {
                                            egui::Color32::from_rgba_unmultiplied(27, 27, 27, 160)
                                        // Dark mode
                                        } else {
                                            egui::Color32::from_rgba_unmultiplied(
                                                248, 248, 248, 160,
                                            ) // Light mode
                                        };
                                        ui.painter().rect_filled(text_rect, 0.0, background_colour);

                                        ui.put(text_rect, text);
                                    }
                                }
                            }
                        });
                    }
                } else {
                    for i in row_range {
                        if let Some(asset) = file_list.get(i) {
                            let alias = config::get_asset_alias(&asset.name);
                            let is_selected = if none_selected && i != 0 {
                                self.selected = Some(i);
                                none_selected = false;
                                true
                            } else {
                                self.selected == Some(i)
                            };

                            if is_selected && self.renaming {
                                self.handle_text_edit(ui, &alias, &asset.name);
                            } else {
                                let full_width = ui.available_width();
                                let desired_size = egui::vec2(full_width, row_height);
                                let (rect, response) =
                                    ui.allocate_exact_size(desired_size, egui::Sense::click());

                                let visuals = ui.visuals().clone();
                                let colours = self.handle_asset_response(
                                    response,
                                    &visuals,
                                    is_selected,
                                    i,
                                    scroll_to,
                                    &mut navigation_accepted,
                                    &mut focus_search_box,
                                    asset.clone(),
                                );

                                let text_colour = colours.1;
                                let background_colour = colours.0;

                                ui.painter().rect_filled(rect, 0.0, background_colour);

                                // // Format metadata
                                // let size = format_size(asset.size);
                                // let modified = if asset.last_modified.is_some() {
                                //     format_modified(asset.last_modified.unwrap())
                                // } else {
                                //     "".to_string()
                                // };

                                // Column positions (add padding)
                                let alias_x = rect.min.x + 5.0;
                                let source_x = rect.min.x + rect.width() * 0.7;

                                // Draw name column
                                ui.painter().text(
                                    egui::pos2(alias_x, rect.min.y),
                                    egui::Align2::LEFT_TOP,
                                    alias,
                                    egui::TextStyle::Body.resolve(ui.style()),
                                    text_colour,
                                );

                                // Draw source indicator badge
                                let source_label = if asset.from_rbx_storage {
                                    locale::get_message(&self.locale, "source-rbx-storage", None)
                                } else if asset.from_sql {
                                    locale::get_message(&self.locale, "source-sql", None)
                                } else if asset.from_file {
                                    locale::get_message(&self.locale, "source-cache", None)
                                } else {
                                    String::new()
                                };

                                if !source_label.is_empty() {
                                    let badge_text = egui::RichText::new(&source_label)
                                        .text_style(egui::TextStyle::Small)
                                        .color(text_colour);

                                    let badge_rect = egui::Rect::from_min_max(
                                        egui::pos2(source_x + 4.0, rect.min.y + 1.0),
                                        egui::pos2(rect.max.x - 4.0, rect.max.y - 1.0),
                                    );

                                    let badge_bg = if visuals.dark_mode {
                                        egui::Color32::from_rgba_unmultiplied(60, 60, 60, 120)
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(220, 220, 220, 120)
                                    };

                                    ui.painter().rect_filled(badge_rect, 4.0, badge_bg);

                                    ui.put(
                                        egui::Rect::from_min_size(
                                            badge_rect.min + egui::vec2(4.0, 0.0),
                                            badge_rect.size(),
                                        ),
                                        egui::Label::new(badge_text).truncate().selectable(false),
                                    );
                                }
                            }
                        }
                    }
                }
            },
        );

        if !navigation_accepted && scroll_to.is_some() {
            // If the keyboard navigation wasn't accepted and there is keyboard navigation then...
            self.selected = None; // Set the selected to none, so it selects something on-screen
        }
    }
}

impl Default for FileListUi {
    fn default() -> Self {
        Self {
            selected: None,
            current_tab: None,
            renaming: false,
            searching: false,
            search_query: "".to_owned(),
            swapping: false,
            swapping_asset: None,
            locale: locale::get_locale(None),
            asset_context_menu_open: None,
            copying: false,
        }
    }
}
