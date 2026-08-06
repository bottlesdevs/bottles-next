use iced::{
    Element, Fill, Subscription, Task, Theme,
    keyboard::{self, key},
    widget::{Space, column, container, image, row, scrollable, text},
};
use next_ui::components::text::TextExt as _;
use next_ui::components::{
    action_row, artwork_card, button, card, cycle_row, expander_row, header_bar,
    info_card::{self, Kind},
    info_row, picker_row, row_group, search, selector_row, status_bar, switcher_row, tabs,
    text_row, title, window_frame,
};
use next_ui::{icons::Icon, theme};

const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const EMPTY_OPTIONS: &[&str] = &[];
const TAB_LABELS: &[&str] = &["Bottles", "Library", "Settings"];
const DLSS_LEVELS: &[&str] = &["Off", "Quality", "Balanced", "Performance"];
const SEARCH_CATALOG: &[(&str, &str, Icon)] = &[
    ("Epic Games Store", "Install", Icon::Arrow),
    ("Epic Fight", "Run", Icon::Play),
    ("GOG Galaxy", "Install", Icon::Arrow),
    ("Steam", "Run", Icon::Play),
];
const LOG: &str = "11:42:12 (INFO) Doing runner update for bottle: games\n\
11:42:12 (INFO) Setting Key Runner=caffe-8.18\n\
11:42:36 (INFO) Using Wine Registry CLI";

fn main() -> iced::Result {
    iced::application(Gallery::default, Gallery::update, Gallery::view)
        .title("Bottles Next component gallery")
        .theme(theme)
        .subscription(Gallery::subscription)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((1200.0, 900.0))
        .decorations(false)
        .transparent(true)
        .run()
}

fn theme(_: &Gallery) -> Theme {
    theme::theme()
}

struct Gallery {
    search: String,
    text_rows: [String; 3],
    selected_option: Option<&'static str>,
    selected_tab: usize,
    switched_on: bool,
    group_switched_on: bool,
    status_expanded: bool,
    value: usize,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            search: String::new(),
            text_rows: std::array::from_fn(|_| String::new()),
            selected_option: None,
            selected_tab: 0,
            switched_on: false,
            group_switched_on: false,
            status_expanded: false,
            value: 1,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    TextRowChanged(usize, String),
    OptionSelected(&'static str),
    TabSelected(usize),
    Switched(bool),
    GroupSwitched(bool),
    Window(window_frame::Action),
    StatusToggled,
    Previous,
    Next,
    MoveFocus(bool),
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(value) => self.search = value,
            Message::TextRowChanged(index, value) => self.text_rows[index] = value,
            Message::OptionSelected(value) => {
                self.selected_option = Some(value);
            }
            Message::TabSelected(index) => self.selected_tab = index,
            Message::Switched(value) => self.switched_on = value,
            Message::GroupSwitched(value) => self.group_switched_on = value,
            Message::Window(action) => return action.task(),
            Message::StatusToggled => self.status_expanded = !self.status_expanded,
            Message::Previous => {
                self.value = (self.value + DLSS_LEVELS.len() - 1) % DLSS_LEVELS.len();
            }
            Message::Next => self.value = (self.value + 1) % DLSS_LEVELS.len(),
            Message::MoveFocus(previous) => {
                return if previous {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Noop => {}
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            } => Some(Message::MoveFocus(modifiers.shift())),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let headings = column![
            text("Heading 1").h1(),
            text("Heading 2").h2(),
            text("Heading 3").h3(),
            text("Heading 4").h4(),
            text("Heading 5").h5(),
        ]
        .spacing(6);

        let titles = row![
            title::Title::new("Title").subtitle("Subtitle"),
            title::Title::new("Title").status("Status"),
        ]
        .spacing(24);

        let buttons = row![
            button::Button::new("Play")
                .icon(Icon::Play)
                .on_press(Message::Noop),
            button::Button::new("Pill").pill().on_press(Message::Noop),
            button::Button::icon_only("Play", Icon::Play).on_press(Message::Noop),
            button::Button::new("Disabled"),
            button::Button::new("Loading")
                .on_press(Message::Noop)
                .loading(true),
        ]
        .spacing(12);

        let cards = column![
            row![
                card::Card::new(
                    column![
                        text("Text card").title(),
                        text("Subtitle").subtitle().muted(),
                        text("Cards accept arbitrary content.").body().muted(),
                    ]
                    .spacing(6),
                )
                .padding(24),
                artwork_card::ArtworkCard::new("Artwork card", "Ready")
                    .menu(
                        artwork_card::CardAction::new("More actions", Icon::EllipsisVertical)
                            .on_press(Message::Noop),
                    )
                    .primary(
                        artwork_card::CardAction::new("Play", Icon::Play).on_press(Message::Noop),
                    )
                    .banner(sample_image()),
                artwork_card::ArtworkCard::new("Program card", "Last played today")
                    .secondary(
                        artwork_card::CardAction::new("Settings", Icon::Gear)
                            .on_press(Message::Noop),
                    )
                    .primary(
                        artwork_card::CardAction::new("Play", Icon::Play)
                            .on_press(Message::Noop)
                            .loading(true),
                    )
                    .banner(sample_image()),
            ]
            .spacing(18),
            row![
                info_card::InfoCard::new(Kind::Hint, "Hint", "Helpful contextual information."),
                info_card::InfoCard::new(Kind::Info, "Info", "General information for the user."),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Error, "Error", "Something needs attention."),
                info_card::InfoCard::new(Kind::Warning, "Warning", "Proceed with care."),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Success, "Success", "The operation completed."),
                Space::new().width(Fill),
            ]
            .spacing(12),
        ]
        .spacing(18);

        let tabs = tabs::Tabs::new(
            TAB_LABELS
                .iter()
                .enumerate()
                .map(|(index, label)| tabs::Tab::new(index, label)),
            Some(self.selected_tab),
            Message::TabSelected,
        );

        let search = column![
            search::Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            ),
            search::Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            )
            .state(self.search_state())
            .footer("Not listed, install manually", Message::Noop),
            search::Search::new(
                "Focus to see loading state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(search::SearchState::Loading),
            search::Search::new(
                "Focus to see error state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(search::SearchState::Error(
                "The catalog could not be loaded"
            )),
        ]
        .spacing(18);

        let selected = self
            .selected_option
            .and_then(|selected| SELECTOR_OPTIONS.iter().find(|option| **option == selected));
        let fields = column![
            text_row::TextRow::new("Input Name", &self.text_rows[0])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(0, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[1])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .secure(true)
                .on_input(|value| Message::TextRowChanged(1, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[2])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(2, value))
                .error(Some("Example validation error")),
            selector_row::SelectorRow::new("Selector Name", SELECTOR_OPTIONS, selected,)
                .on_selected(Message::OptionSelected)
                .placeholder("Placeholder")
                .icon(Icon::Person),
            selector_row::SelectorRow::new("Empty selector", EMPTY_OPTIONS, None)
                .placeholder("No options available"),
            action_row::ActionRow::new("Title", action_row::State::Ready(Message::Noop))
                .description("Description"),
            action_row::ActionRow::new("Unavailable action", action_row::State::Disabled)
                .description("This action cannot currently run"),
            info_row::InfoRow::new("Title")
                .description("Description")
                .icon(Icon::Timer),
            switcher_row::SwitcherRow::new("Title", self.switched_on)
                .on_toggle(Message::Switched)
                .description("Description"),
            cycle_row::CycleRow::new("DLSS Level", DLSS_LEVELS[self.value])
                .on_previous(Message::Previous)
                .on_next(Message::Next),
            picker_row::PickerRow::new("Title")
                .description("Choose the location")
                .on_press(Message::Noop),
            expander_row::ExpanderRow::with_header(
                switcher_row::SwitcherRow::new("FSR", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("FidelityFX Super Resolution"),
            )
            .columns(2)
            .add(
                action_row::ActionRow::new("Quality", action_row::State::Ready(Message::Noop),)
                    .description("Balanced"),
            )
            .add(
                cycle_row::CycleRow::new("Sharpening", "5")
                    .on_previous(Message::Previous)
                    .on_next(Message::Next),
            )
            .content_enabled(self.switched_on),
        ]
        .spacing(24);

        let row_group = row_group::RowGroup::new()
            .title("Graphics")
            .description("Rows wrap according to the configured column count.")
            .columns(2)
            .add(
                switcher_row::SwitcherRow::new("DLSS", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("Deep Learning Super Sampling"),
            )
            .add(
                picker_row::PickerRow::new("Shader directory")
                    .description("Choose the location")
                    .on_press(Message::Noop),
            )
            .add(
                action_row::ActionRow::new("Discrete GPU", action_row::State::Ready(Message::Noop))
                    .description("Configure graphics adapter"),
            )
            .add(
                expander_row::ExpanderRow::with_header(
                    switcher_row::SwitcherRow::new("FSR", self.group_switched_on)
                        .on_toggle(Message::GroupSwitched)
                        .description("FidelityFX Super Resolution"),
                )
                .columns(2)
                .add(
                    action_row::ActionRow::new("Quality", action_row::State::Ready(Message::Noop))
                        .description("Balanced"),
                )
                .add(
                    cycle_row::CycleRow::new("Sharpening", DLSS_LEVELS[self.value])
                        .on_previous(Message::Previous)
                        .on_next(Message::Next),
                )
                .content_enabled(self.group_switched_on),
            );

        let multiple_expanders = row_group::RowGroup::new()
            .title("Non-overlapping expanders")
            .description("Both expanders can remain open because their panels do not overlap")
            .columns(3)
            .add(
                expander_row::ExpanderRow::new("First expander")
                    .description("One-column panel")
                    .add(
                        action_row::ActionRow::new(
                            "First action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("Inside the first column"),
                    ),
            )
            .add(
                expander_row::ExpanderRow::new("Second expander")
                    .description("Two-column panel")
                    .columns(2)
                    .add(
                        action_row::ActionRow::new(
                            "Second action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("First panel column"),
                    )
                    .add(
                        action_row::ActionRow::new(
                            "Third action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("Second panel column"),
                    ),
            )
            .add(
                action_row::ActionRow::new(
                    "Independent action",
                    action_row::State::Ready(Message::Noop),
                )
                .description("Beside the two expanders"),
            );

        let expander_matrix = row_group::RowGroup::new()
            .title("2 × 2 expander grid")
            .description("Opening a sibling closes the overlapping panel on the same grid line")
            .columns(2)
            .add(action_grid_expander("Expander A"))
            .add(action_grid_expander("Expander B"))
            .add(action_grid_expander("Expander C"))
            .add(action_grid_expander("Expander D"));

        let status = column![
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::StatusState::Running,)
                .log(LOG),
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::StatusState::Stopped,)
                .log(LOG)
                .expanded(self.status_expanded)
                .on_toggle(Message::StatusToggled),
            status_bar::StatusBar::<Message>::new(
                "Win64",
                "soda-7.0.9",
                status_bar::StatusState::Starting,
            ),
            status_bar::StatusBar::<Message>::new(
                "Win64",
                "soda-7.0.9",
                status_bar::StatusState::Failed,
            ),
        ]
        .spacing(18);

        let header = header_bar::HeaderBar::new(Message::Window).middle(
            iced::widget::container(
                search::Search::new(
                    "Search for software and games…",
                    &self.search,
                    Message::SearchChanged,
                )
                .state(self.search_state()),
            )
            .padding(10)
            .width(370),
        );

        let gallery = scrollable(
            container(
                column![
                    section("Headings", headings),
                    section("Title", titles),
                    section("Buttons", buttons),
                    section("Cards", cards),
                    section("Tabs", tabs),
                    section("Search", search),
                    section("Rows", fields),
                    section("Row group", row_group),
                    section("Overlap-aware expanders", multiple_expanders),
                    section("Expander matrix", expander_matrix),
                    section("Status bar", status),
                ]
                .spacing(12)
                .padding(12)
                .width(Fill)
                .max_width(1150),
            )
            .center_x(Fill),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .margin(12),
        ))
        .style(theme::scrollbar)
        .width(Fill)
        .height(Fill);

        window_frame::WindowFrame::new(column![header, gallery], Message::Window).into()
    }

    fn search_state(&self) -> search::SearchState<'_, Message> {
        let query = self.search.trim().to_lowercase();

        if query.is_empty() {
            return search::SearchState::Hidden;
        }

        let results: Vec<_> = SEARCH_CATALOG
            .iter()
            .filter(|(title, _, _)| title.to_lowercase().contains(&query))
            .map(|(title, action, action_icon)| {
                search::SearchResult::new(*title, *title, Message::Noop)
                    .icon(Icon::Bottles)
                    .action(action, *action_icon, Message::Noop)
            })
            .collect();

        if results.is_empty() {
            search::SearchState::Empty
        } else {
            search::SearchState::Results(results)
        }
    }
}

fn action_grid_expander(title: &'static str) -> expander_row::ExpanderRow<'static, Message> {
    ["Action 1", "Action 2", "Action 3", "Action 4"]
        .into_iter()
        .fold(
            expander_row::ExpanderRow::new(title)
                .description("Contains four actions")
                .columns(2),
            |expander, title| {
                expander.add(
                    action_row::ActionRow::new(title, action_row::State::Ready(Message::Noop))
                        .description("Available action"),
                )
            },
        )
}

fn section<'a>(label: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(column![text(label).title(), content.into()].spacing(12))
        .width(Fill)
        .padding(24)
        .style(theme::panel)
        .into()
}

fn sample_image() -> image::Handle {
    image::Handle::from_rgba(
        2,
        2,
        vec![
            51, 53, 71, 255, 53, 71, 51, 255, 92, 63, 63, 255, 107, 93, 71, 255,
        ],
    )
}
