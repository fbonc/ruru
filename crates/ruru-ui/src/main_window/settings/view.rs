use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, scrollable, space, text, text_input,
};
use iced::{Center, Element, Fill};
use ruru_types::ProviderId;

use super::super::style;
use super::{Input, ProviderView, Section, Settings};

#[derive(Clone, PartialEq, Eq)]
struct ProviderChoice {
    id: ProviderId,
    name: String,
}

impl std::fmt::Display for ProviderChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.name)
    }
}

impl ProviderView {
    fn detail_view(&self) -> Element<'_, Input> {
        let id = self.id.clone();
        let key_input = text_input("Paste an API key", &self.api_key_draft)
            .secure(true)
            .on_input(move |value| Input::ApiKeyChanged(id.clone(), value))
            .on_submit(Input::SaveKeyRequested(self.id.clone()))
            .padding(10)
            .style(style::key_input);
        let save_key = button(text(if self.credential_present {
            "Update key"
        } else {
            "Save key"
        }))
        .on_press_maybe(
            (!self.api_key_draft.trim().is_empty())
                .then(|| Input::SaveKeyRequested(self.id.clone())),
        )
        .padding([10, 14])
        .style(style::action_button);

        let key_controls = column![
            text("API key").size(style::SUBHEADING_SIZE),
            text(if self.credential_present {
                "API keys are saved in Keychain"
            } else {
                "Add an API key to discover models"
            })
            .size(style::BODY_TEXT_SIZE)
            .color(style::MUTED_COLOR),
            row![key_input, save_key].spacing(10).align_y(Center),
        ]
        .spacing(10);

        let model_header = row![
            text("Available models").size(style::SUBHEADING_SIZE),
            space().width(Fill),
            button(text("Refresh"))
                .on_press_maybe(
                    (self.credential_present && !self.discovering)
                        .then(|| Input::RefreshModelsRequested(self.id.clone()))
                )
                .padding([7, 10])
                .style(style::subtle_button),
        ]
        .align_y(Center);
        let mut model_choices = column![].spacing(10);

        if self.discovering {
            model_choices =
                model_choices.push(text("Discovering models…").color(style::MUTED_COLOR));
        } else if self.discovered_models.is_empty() {
            model_choices = model_choices
                .push(text("Refresh to discover available models.").color(style::MUTED_COLOR));
        } else {
            for model in &self.discovered_models {
                let id = self.id.clone();
                let model_id = model.clone();
                model_choices = model_choices.push(
                    checkbox(self.selected_models.contains(model))
                        .label(model.as_str())
                        .on_toggle(move |enabled| {
                            Input::ModelToggled(id.clone(), model_id.clone(), enabled)
                        })
                        .size(17)
                        .style(style::provider_checkbox),
                );
            }
        }

        let model_list: Element<'_, Input> =
            if self.discovered_models.len() > 8 && !self.discovering {
                scrollable(container(model_choices).padding(iced::Padding {
                    right: style::SCROLL_CONTENT_GAP,
                    ..Default::default()
                }))
                .height(style::MODEL_LIST_HEIGHT)
                .width(Fill)
                .style(crate::style::scroll)
                .into()
            } else {
                model_choices.into()
            };
        let model_list = container(model_list)
            .padding(12)
            .width(Fill)
            .max_width(style::MODEL_LIST_WIDTH)
            .style(style::model_list);
        let model_header = container(model_header)
            .width(Fill)
            .max_width(style::MODEL_LIST_WIDTH);
        let models = column![model_header, model_list].spacing(10);

        let mut content = column![key_controls]
            .spacing(style::SECTION_SPACING)
            .width(Fill);
        if let Some(error) = &self.error {
            content = content.push(text(error).color(style::DANGER_COLOR));
        }
        if self.credential_present {
            content = content
                .push(rule::horizontal(1).style(style::sidebar_rule))
                .push(models);
        }
        content.into()
    }
}

impl Settings {
    pub fn view(&self) -> Element<'_, Input> {
        let navigation = column![
            button(text("Providers & Models").size(style::SECTION_TEXT_SIZE))
                .on_press(Input::SectionSelected(Section::Providers))
                .width(Fill)
                .padding([10, 12])
                .style(style::selected_section),
        ]
        .width(Fill);

        let content = column![self.provider_view()].width(Fill);
        row![
            container(navigation)
                .padding(iced::Padding {
                    top: style::CONTENT_SPACING,
                    ..Default::default()
                })
                .width(style::SIDEBAR_WIDTH),
            rule::vertical(1).style(style::sidebar_rule),
            container(
                scrollable(container(content).padding(iced::Padding {
                    right: style::SCROLL_CONTENT_GAP,
                    ..Default::default()
                }))
                .id(self.scroll_id.clone())
                .width(Fill)
                .height(Fill)
                .style(crate::style::scroll),
            )
            .padding(iced::Padding {
                top: style::CONTENT_SPACING,
                ..Default::default()
            })
            .width(Fill)
            .height(Fill),
        ]
        .spacing(style::SETTINGS_COLUMN_SPACING)
        .height(Fill)
        .into()
    }
    fn provider_view(&self) -> Element<'_, Input> {
        let mut content: iced::widget::Column<'_, Input> = column![
            text("Providers & Models").size(style::PAGE_HEADING_SIZE),
            text("Connect providers and choose which models appear in the overlay.")
                .size(style::BODY_TEXT_SIZE)
                .color(style::MUTED_COLOR),
        ]
        .spacing(style::SECTION_SPACING)
        .width(Fill);

        let mut selected = column![text("Overlay models").size(style::SUBHEADING_SIZE)].spacing(10);
        let mut has_selected_models = false;
        for provider in self
            .providers
            .iter()
            .filter(|provider| provider.credential_present)
        {
            for model in &provider.selected_models {
                has_selected_models = true;
                selected = selected.push(
                    row![
                        text(model).width(Fill),
                        text(&provider.name).color(style::MUTED_COLOR),
                        button(text("×"))
                            .on_press(Input::ModelToggled(
                                provider.id.clone(),
                                model.clone(),
                                false
                            ))
                            .padding([4, 8])
                            .style(style::subtle_button),
                    ]
                    .spacing(12)
                    .align_y(Center),
                );
            }
        }
        if !has_selected_models {
            selected = selected.push(text("No models selected yet.").color(style::MUTED_COLOR));
        }
        content = content.push(
            container(selected)
                .padding(style::SECTION_PADDING)
                .width(Fill)
                .style(style::provider_card),
        );

        let choices = self
            .providers
            .iter()
            .filter(|provider| !provider.credential_present)
            .map(|provider| ProviderChoice {
                id: provider.id.clone(),
                name: provider.name.clone(),
            })
            .collect::<Vec<_>>();
        let add_control: Element<'_, Input> = if choices.is_empty() {
            button(text("Add a provider"))
                .padding([6, 12])
                .style(style::action_button)
                .into()
        } else {
            pick_list(choices, None::<ProviderChoice>, |choice| {
                Input::ProviderSelected(choice.id)
            })
            .placeholder("Add a provider")
            .padding([6, 10])
            .style(style::add_provider_picker)
            .menu_style(style::add_provider_menu)
            .into()
        };

        let mut providers = column![
            row![
                text("Providers").size(style::SUBHEADING_SIZE),
                space().width(Fill),
                add_control,
            ]
            .align_y(Center),
        ]
        .spacing(12);

        let mut has_provider_row = false;
        for provider in self.providers.iter().filter(|provider| {
            provider.credential_present || self.active_provider.as_ref() == Some(&provider.id)
        }) {
            has_provider_row = true;
            let expanded = self.active_provider.as_ref() == Some(&provider.id);
            let status: Element<'_, Input> = if provider.credential_present {
                button(text("×").size(style::SUBHEADING_SIZE))
                    .on_press(Input::RemoveKeyRequested(provider.id.clone()))
                    .padding([2, 6])
                    .style(style::subtle_button)
                    .into()
            } else {
                text("Not connected").color(style::MUTED_COLOR).into()
            };
            providers = providers.push(
                row![
                    button(
                        row![
                            text(if expanded { "▾" } else { "▸" }),
                            text(&provider.name).width(Fill),
                        ]
                        .spacing(12)
                        .align_y(Center)
                    )
                    .on_press(Input::ProviderRowToggled(provider.id.clone()))
                    .width(Fill)
                    .padding([8, 4])
                    .style(style::subtle_button),
                    status,
                ]
                .spacing(12)
                .align_y(Center),
            );
            if expanded {
                providers = providers.push(container(provider.detail_view()).width(Fill).padding(
                    iced::Padding {
                        top: 4.0,
                        right: 4.0,
                        bottom: 12.0,
                        left: 28.0,
                    },
                ));
            }
        }
        if !has_provider_row {
            providers = providers.push(text("No providers connected.").color(style::MUTED_COLOR));
        }
        content = content.push(
            container(providers)
                .padding(style::SECTION_PADDING)
                .width(Fill)
                .style(style::provider_card),
        );
        if let Some(error) = &self.error {
            content = content.push(text(error).color(style::DANGER_COLOR));
        }
        container(content).width(Fill).into()
    }
}
