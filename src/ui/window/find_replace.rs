use crate::app::Message;
use iced::widget::{button, column, row, text, text_input, Space};
use iced::{Element, Fill};
use rust_i18n::t;

pub const FIND_INPUT_ID: &str = "find-replace-search";

pub fn view_window<'a>(
    search: &'a str,
    replacement: &'a str,
    status: &'a str,
) -> Element<'a, Message> {
    let find_input = text_input(t!("Text to find").as_ref(), search)
        .id(iced::widget::Id::new(FIND_INPUT_ID))
        .on_input(Message::FindReplaceSearchChanged)
        .on_submit(Message::FindReplaceNext)
        .padding([6, 8])
        .size(13);
    let replacement_input = text_input(t!("Replacement text").as_ref(), replacement)
        .on_input(Message::FindReplaceReplacementChanged)
        .padding([6, 8])
        .size(13);

    let enabled = !search.trim().is_empty();
    let action = |label: std::borrow::Cow<'static, str>, message: Message| {
        let button = button(text(label).size(12)).padding([6, 12]);
        if enabled {
            button.on_press(message)
        } else {
            button
        }
    };

    column![
        row![
            text(t!("Find:")).size(12).width(90),
            find_input.width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        row![
            text(t!("Replace with:")).size(12).width(90),
            replacement_input.width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        text(t!(
            "Searches Text, MText, Attribute Definitions, and block attribute values."
        ))
        .size(11),
        text(status).size(11),
        row![
            Space::new().width(Fill),
            button(text(t!("Close")).size(12))
                .on_press(Message::CloseModal)
                .padding([6, 12])
                .style(button::secondary),
            action(t!("Replace"), Message::FindReplaceOne).style(button::secondary),
            action(t!("Replace All"), Message::FindReplaceAll).style(button::danger),
            action(t!("Find Next"), Message::FindReplaceNext).style(button::primary),
        ]
        .spacing(8)
        .align_y(iced::Center),
    ]
    .spacing(10)
    .padding(12)
    .width(Fill)
    .into()
}
