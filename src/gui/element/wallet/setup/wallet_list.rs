use {
    super::super::super::{DEFAULT_FONT_SIZE, DEFAULT_HEADER_FONT_SIZE, DEFAULT_PADDING},
    crate::gui::{style, GrinGui, Interaction, Message},
    crate::localization::localized_string,
    crate::Result,
    grin_gui_core::config::Config,
    grin_gui_core::{
        theme::ColorPalette,
        wallet::{create_grin_wallet_path, ChainTypes},
    },
    iced::{
        alignment, button, scrollable, Alignment, Button, Checkbox, Column, Command, Container,
        Element, Length, Row, Scrollable, Space, Text,
    },
    native_dialog::FileDialog,
    std::path::PathBuf,
};

use grin_gui_widgets::{table_row::StyleSheet, TableRow};

pub struct StateContainer {
    pub back_button_state: button::State,
    selected_wallet_index: usize,
    load_wallet_button_state: button::State,
    select_folder_button_state: button::State,
    new_wallet_button_state: button::State,
    scrollable_state: scrollable::State,
}

impl Default for StateContainer {
    fn default() -> Self {
        Self {
            back_button_state: Default::default(),
            selected_wallet_index: 0,
            load_wallet_button_state: Default::default(),
            select_folder_button_state: Default::default(),
            new_wallet_button_state: Default::default(),
            scrollable_state: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LocalViewInteraction {
    Back,
    WalletRowSelect(bool, usize),
    LoadWallet(usize),
    LocateWallet,
    CreateWallet,
}

pub fn handle_message<'a>(
    grin_gui: &mut GrinGui,
    message: LocalViewInteraction,
) -> Result<Command<Message>> {
    match message {
        LocalViewInteraction::Back => {
            grin_gui.wallet_state.setup_state.mode = super::Mode::Init;
        }
        LocalViewInteraction::WalletRowSelect(is_selected, index) => {
            if is_selected {
                grin_gui
                    .wallet_state
                    .setup_state
                    .setup_wallet_list_state
                    .selected_wallet_index = index;
            }
        }
        LocalViewInteraction::LoadWallet(index) => {
            grin_gui.config.current_wallet_index = Some(index);
            grin_gui.wallet_state.mode = crate::gui::element::wallet::Mode::Operation;
        }
        LocalViewInteraction::LocateWallet => {
            match FileDialog::new().show_open_single_file() {
                Ok(path) => {
                    match path {
                        Some(d) => {
                            match validate_directory(d) {
                                Ok(wallet_was_imported) => {}
                                Err(err) => {
                                    // tell the user why this directory failed
                                }
                            }
                        }
                        None => {}
                    }
                }
                Err(e) => {
                    log::debug!("wallet_list.rs::LocalViewInteraction::LocateWallet {}", e);
                }
            };
        }
        LocalViewInteraction::CreateWallet => {
            let state = &mut grin_gui.wallet_state.setup_state;
            let config = &grin_gui.config;
            let wallet_default_name = localized_string("wallet-default-name");
            let mut wallet_display_name = wallet_default_name.clone();
            let mut i = 1;

            // wallet display name must be unique: i.e. Default 1, Default 2, ...
            while let Some(_) = config
                .wallets
                .iter()
                .find(|wallet| wallet.display_name == wallet_display_name)
            {
                wallet_display_name = format!("{} {}", wallet_default_name, i);
                i += 1;
            }

            // i.e. default_1, default_2, ...
            let wallet_dir: String = str::replace(&wallet_display_name.to_lowercase(), " ", "_");

            state
                .setup_wallet_state
                .advanced_options_state
                .top_level_directory = create_grin_wallet_path(&ChainTypes::Mainnet, &wallet_dir);

            grin_gui.wallet_state.mode =
                crate::gui::element::wallet::Mode::CreateWallet(wallet_display_name);
        }
    }

    Ok(Command::none())
}

struct DirectoryValidationError;

fn validate_directory(_d: PathBuf) -> Result<bool, DirectoryValidationError> {
    Ok(true)
}

pub fn data_container<'a>(
    color_palette: ColorPalette,
    state: &'a mut StateContainer,
    config: &Config,
) -> Container<'a, Message> {
    let title = Text::new(localized_string("wallet-list")).size(DEFAULT_HEADER_FONT_SIZE);
    let title_container =
        Container::new(title).style(style::BrightBackgroundContainer(color_palette));

    let new_wallet_button: Element<Interaction> = Button::new(
        &mut state.new_wallet_button_state,
        Text::new(localized_string("create-wallet")).size(DEFAULT_FONT_SIZE),
    )
    .style(style::DefaultButton(color_palette))
    .on_press(Interaction::WalletListWalletViewInteraction(
        LocalViewInteraction::CreateWallet,
    ))
    .into();

    let button_row = Row::new()
        .push(new_wallet_button.map(Message::Interaction))
        .spacing(0);

    let segmented_mode_container = Container::new(button_row)
        .style(style::SegmentedContainer(color_palette));

    let mut header_row = Row::new()
        .height(Length::Units(50))
        .push(Space::new(Length::Units(DEFAULT_PADDING), Length::Units(1)))
        .push(Space::new(Length::Units(10), Length::Units(0)))
        .push(title_container)
        .push(Space::new(Length::Fill, Length::Units(0)))
        .push(segmented_mode_container)
        .push(Space::new(
            Length::Units(DEFAULT_PADDING + 5),
            Length::Units(0),
        ))
        .align_items(Alignment::Center);

    let mut wrapper_column = Column::new().height(Length::Fill).push(header_row);

    let name_header = Text::new(localized_string("Name")).size(DEFAULT_HEADER_FONT_SIZE);

    let name_header_container =
        Container::new(name_header).style(style::NormalBackgroundContainer(color_palette));

    let chain_header = Text::new(localized_string("Chain")).size(DEFAULT_HEADER_FONT_SIZE);

    let chain_header_container =
        Container::new(chain_header).style(style::NormalBackgroundContainer(color_palette));

    let directory_header = Text::new(localized_string("Location")).size(DEFAULT_HEADER_FONT_SIZE);

    let directory_header_container =
        Container::new(directory_header).style(style::NormalBackgroundContainer(color_palette));

    let header_row = Row::new()
        .push(name_header_container)
        .push(Space::new(Length::Units(85), Length::Units(0)))
        .push(chain_header_container)
        .push(Space::new(Length::Units(135), Length::Units(0)))
        .push(directory_header_container);

    let mut wallet_rows: Vec<_> = vec![];
    for (pos, w) in config.wallets.iter().enumerate() {
        let checkbox = Checkbox::new(state.selected_wallet_index == pos, "", move |b| {
            Message::Interaction(Interaction::WalletListWalletViewInteraction(
                LocalViewInteraction::WalletRowSelect(b, pos),
            ))
        })
        .style(style::DefaultCheckbox(color_palette))
        .text_size(DEFAULT_FONT_SIZE)
        .spacing(10);

        let wallet_name = Text::new(w.display_name.clone()).size(DEFAULT_HEADER_FONT_SIZE);
        let chain_name = Text::new(w.chain_type.shortname()).size(DEFAULT_HEADER_FONT_SIZE);

        let wallet_name_container =
            Container::new(wallet_name).style(style::HoverableForegroundContainer(color_palette));

        let wallet_chain_container =
            Container::new(chain_name).style(style::HoverableForegroundContainer(color_palette));

        let tld_string = match &w.tld {
            Some(path_buf) => path_buf.display().to_string(),
            None => String::from("Unknown"),
        };
        let wallet_directory = Text::new(tld_string).size(DEFAULT_HEADER_FONT_SIZE);

        let wallet_directory_container =
            Container::new(wallet_directory).style(style::HoverableForegroundContainer(color_palette));

        let wallet_row = Row::new()
            .push(checkbox)
            .push(
                Column::new()
                    .push(wallet_name_container)
                    .width(Length::Units(80)),
            )
            .push(
                Column::new()
                    .push(wallet_chain_container)
                    .width(Length::Units(60)),
            )
            .push(Space::new(Length::Units(58), Length::Units(0)))
            .push(wallet_directory_container);

        let mut style =  style::TableRow(color_palette);
        let mut table_row = TableRow::new(wallet_row)
            .padding(iced::Padding::from(2))
            .width(Length::Fill)
            .style(style);
            // .on_press(move |e| {
            //     println!("table_row on press {}", pos);

            //     Message::Interaction(Interaction::WalletListWalletViewInteraction(
            //         LocalViewInteraction::WalletRowSelect(true, pos),
            //     ))
            // });

        wallet_rows.push(table_row.into());
    }

    let c = Container::new(
        Column::new()
            .push(Space::new(Length::Units(0), Length::Units(3)))
            .push(Column::with_children(wallet_rows))
            .push(Space::new(Length::Units(0), Length::Units(3))),
    )
    .style(style::ChannelBadge(color_palette))
    .padding(iced::Padding::from(DEFAULT_PADDING));

    let mut pad_right = iced::Padding::ZERO;
    pad_right.right = 15;
    let wallet_column = Column::new()
        .push(header_row)
        .push(Space::new(Length::Units(0), Length::Units(5)))
        .push(c)
        .padding(pad_right);

    let load_wallet_button_container =
        Container::new(Text::new(localized_string("load-wallet")).size(DEFAULT_FONT_SIZE))
            .height(Length::Units(20))
            .align_y(alignment::Vertical::Center)
            .align_x(alignment::Horizontal::Center);

    let load_wallet_button: Element<Interaction> = Button::new(
        &mut state.load_wallet_button_state,
        load_wallet_button_container,
    )
    .style(style::DefaultBoxedButton(color_palette))
    .on_press(Interaction::WalletListWalletViewInteraction(
        LocalViewInteraction::LoadWallet(state.selected_wallet_index),
    ))
    .into();

    let select_folder_button_container =
        Container::new(Text::new(localized_string("select-other")).size(DEFAULT_FONT_SIZE))
            .height(Length::Units(20))
            .align_y(alignment::Vertical::Center)
            .align_x(alignment::Horizontal::Center);

    let select_other_button: Element<Interaction> = Button::new(
        &mut state.select_folder_button_state,
        select_folder_button_container,
    )
    .style(style::DefaultBoxedButton(color_palette))
    .on_press(Interaction::WalletListWalletViewInteraction(
        LocalViewInteraction::LocateWallet,
    ))
    .into();

    let button_row = Row::new()
        .push(load_wallet_button.map(Message::Interaction))
        .push(Space::new(Length::Units(DEFAULT_PADDING), Length::Units(0)))
        .push(select_other_button.map(Message::Interaction));

    let parent = Column::new()
        .push(Space::new(Length::Units(0), Length::Units(15)))
        .push(wallet_column)
        .push(Space::new(Length::Units(0), Length::Units(DEFAULT_PADDING)))
        .push(button_row);

    let mut scrollable = Scrollable::new(&mut state.scrollable_state)
        .spacing(1)
        .height(Length::FillPortion(1))
        .style(style::Scrollable(color_palette));

    scrollable = scrollable.height(Length::Fill).width(Length::Fill);
    scrollable = scrollable.push(parent);

    let col = Column::new()
        .push(Space::new(Length::Units(0), Length::Units(10)))
        .push(scrollable)
        .push(Space::new(Length::Units(0), Length::Units(20)));
    let row = Row::new()
        .push(Space::new(Length::Units(20), Length::Units(0)))
        .push(col);

    // Returns the final container.
    let content = Container::new(row)
        .center_x()
        .width(Length::Fill)
        .height(Length::Shrink)
        .style(style::NormalBackgroundContainer(color_palette));

    wrapper_column = wrapper_column.push(content);
    Container::new(wrapper_column)
}
