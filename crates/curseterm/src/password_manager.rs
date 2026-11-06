use std::collections::HashMap;
use std::io::Read;

use barrier_kit::compact::IconButton;
use barrier_kit::controls::{Button, ButtonStyle, Checkbox, SecureField, TextField};
use barrier_kit::password::PasswordField;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::views::feedback::sheet;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit::views::table::Table;
use barrier_kit_next::secrets::{Error, Item, Secrets};
use barrier_kit_next::tokens::PASSWORD_MANAGER;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Direction, Size};
use vow_design::{CAPTION, HEADLINE, SPACE_1, SPACE_2};
use zeroize::Zeroizing;

use crate::localize::tr;

// KeychainPasswordDataSource.swift: every account is one item of this service.
const SERVICE: &str = "CurseTerm";
// iTermPasswordManagerWindowController.m +randomPassword.
const PASSWORD_CHARACTERS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+{}|:<>?,./;'[]-=";

#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    item: Item,
    pub name: String,
    pub user: String,
}

fn attributes<'a>(name: &'a str, user: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("service", SERVICE), ("account", name), ("user", user)])
}

fn message(error: Error) -> String {
    match error {
        Error::Locked => tr!("password-manager-locked").to_string(),
        Error::Bus(error) => error.to_string(),
    }
}

fn load() -> Result<Vec<Account>, String> {
    let secrets = Secrets::open().map_err(message)?;
    let mut accounts: Vec<Account> = secrets
        .search(&HashMap::from([("service", SERVICE)]))
        .map_err(message)?
        .into_iter()
        .map(|item| Account {
            name: item
                .attributes
                .get("account")
                .cloned()
                .unwrap_or_else(|| item.label.clone()),
            user: item.attributes.get("user").cloned().unwrap_or_default(),
            item,
        })
        .collect();
    accounts.sort_by_key(|account| account.name.to_lowercase());
    Ok(accounts)
}

pub fn password(account: &Account) -> Result<Zeroizing<String>, String> {
    let secret = Secrets::open()
        .and_then(|secrets| secrets.secret(&account.item))
        .map_err(message)?;
    Ok(Zeroizing::new(
        String::from_utf8_lossy(&secret).into_owned(),
    ))
}

// +randomPassword: uniform over the characters, from the kernel's random source.
fn random_password() -> Zeroizing<String> {
    let characters: Vec<char> = PASSWORD_CHARACTERS.chars().collect();
    let limit = (256 / characters.len() * characters.len()) as u8;
    let mut password = Zeroizing::new(String::new());
    let Ok(mut random) = std::fs::File::open("/dev/urandom") else {
        return password;
    };
    let mut byte = [0u8];
    while password.chars().count() < PASSWORD_MANAGER.generated_length {
        if random.read_exact(&mut byte).is_err() {
            break;
        }
        if byte[0] < limit {
            password.push(characters[byte[0] as usize % characters.len()]);
        }
    }
    password
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    List,
    Add,
    ChangePassword(usize),
    ConfirmDelete(usize),
}

#[derive(PartialEq)]
pub struct PasswordManager {
    pub send_return: bool,
    pub broadcast: bool,
    pub select: Option<String>,
    pub on_send_return: Callback<bool, ()>,
    pub on_broadcast: Callback<bool, ()>,
    pub on_user_name: Callback<String, ()>,
    pub on_password: Callback<Account, ()>,
    pub on_close: Callback<(), ()>,
}

// iTermPasswordManager.xib: search, the accounts table, + − Edit, Broadcast, and Close, Enter User
// Name and Enter Password; the settings menu's Copy and Reveal Password, Reload Items and
// Automatically Send Return Key after Password sit as buttons and a checkbox.
impl Component for PasswordManager {
    fn render(&self) -> impl IntoElement {
        let mut accounts = use_state(Vec::<Account>::new);
        let mut problem = use_state(|| None::<String>);
        let query = use_state(String::new);
        let mut selected = use_state(|| None::<usize>);
        let mut mode = use_state(|| Mode::List);
        let mut revealed = use_state(|| None::<Zeroizing<String>>);
        let name = use_state(String::new);
        let user = use_state(String::new);
        let mut secret = use_state(PasswordField::default);
        let wanted = self.select.clone();
        let reload = move || match load() {
            Ok(found) => {
                if let Some(wanted) = &wanted {
                    selected.set(found.iter().position(|account| account.name == *wanted));
                }
                accounts.set(found);
                problem.set(None);
            }
            Err(error) => problem.set(Some(error)),
        };
        use_hook({
            let mut reload = reload.clone();
            move || reload()
        });
        let typed = query.read().to_lowercase();
        let shown: Vec<usize> = (0..accounts.read().len())
            .filter(|index| {
                let account = &accounts.read()[*index];
                typed.is_empty()
                    || account.name.to_lowercase().contains(&typed)
                    || account.user.to_lowercase().contains(&typed)
            })
            .collect();
        let chosen = selected.read().filter(|index| shown.contains(index));
        let current = chosen.and_then(|index| accounts.read().get(index).cloned());
        let palette = Palette::current();
        // freya-core 0.4.3 runner.rs:793 unwraps a scope that a component in the same place of
        // another mode left behind; a key per mode remounts the body instead.
        let mode_key = match *mode.read() {
            Mode::List => 0,
            Mode::Add => 1,
            Mode::ChangePassword(_) => 2,
            Mode::ConfirmDelete(_) => 3,
        };
        let body = match *mode.read() {
            Mode::List => {
                let rows = shown
                    .iter()
                    .map(|index| {
                        let account = &accounts.read()[*index];
                        let password = match (&*revealed.read(), chosen) {
                            (Some(password), Some(at)) if at == *index => password.to_string(),
                            _ => "••••••••".to_string(),
                        };
                        vec![account.name.clone(), account.user.clone(), password]
                    })
                    .collect();
                let positions = shown.clone();
                rect()
                    .width(Size::fill())
                    .direction(Direction::Vertical)
                    .spacing(SPACE_2)
                    .child(
                        TextField::new(query, tr!("password-manager-search")).width(Size::fill()),
                    )
                    .child(
                        rect()
                            .height(Size::px(PASSWORD_MANAGER.table_height))
                            .child(
                                Table::new(
                                    tr!("password-manager"),
                                    vec![tr!("account-name"), tr!("user-name"), tr!("password")],
                                )
                                .rows(rows)
                                .selected(
                                    chosen
                                        .and_then(|at| shown.iter().position(|index| *index == at)),
                                )
                                .on_select(move |row: usize| {
                                    revealed.set(None);
                                    selected.set(positions.get(row).copied());
                                }),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .direction(Direction::Horizontal)
                            .cross_align(Alignment::Center)
                            .spacing(SPACE_1)
                            .child(
                                IconButton::new(Symbol::Plus, tr!("password-manager-add"))
                                    .on_press(move |_| {
                                        let (mut name, mut user) = (name, user);
                                        name.set(String::new());
                                        user.set(String::new());
                                        secret.set(PasswordField::default());
                                        mode.set(Mode::Add);
                                    }),
                            )
                            .child(
                                IconButton::new(Symbol::Minus, tr!("password-manager-remove"))
                                    .disabled(chosen.is_none())
                                    .on_press(move |_| {
                                        if let Some(index) = chosen {
                                            mode.set(Mode::ConfirmDelete(index));
                                        }
                                    }),
                            )
                            .child(
                                Button::new(tr!("password-manager-edit"))
                                    .disabled(chosen.is_none())
                                    .on_press(move |_| {
                                        if let Some(index) = chosen {
                                            secret.set(PasswordField::default());
                                            mode.set(Mode::ChangePassword(index));
                                        }
                                    }),
                            )
                            .child(
                                Button::new(tr!("reveal-password"))
                                    .disabled(chosen.is_none())
                                    .on_press({
                                        let current = current.clone();
                                        move |_| {
                                            if let Some(account) = &current {
                                                match password(account) {
                                                    Ok(found) => revealed.set(Some(found)),
                                                    Err(error) => problem.set(Some(error)),
                                                }
                                            }
                                        }
                                    }),
                            )
                            .child(Button::new(tr!("reload-items")).on_press({
                                let mut reload = reload.clone();
                                move |_| reload()
                            })),
                    )
                    .into_element()
            }
            Mode::Add | Mode::ChangePassword(_) => {
                let adding = *mode.read() == Mode::Add;
                let generate = move |_| {
                    let mut field = PasswordField::default();
                    random_password().chars().for_each(|c| field.insert(c));
                    secret.set(field);
                };
                let section = if adding {
                    Section::new()
                        .row(
                            Row::new(tr!("account-name"))
                                .control(TextField::new(name, tr!("required"))),
                        )
                        .row(
                            Row::new(tr!("user-name"))
                                .control(TextField::new(user, tr!("user-name"))),
                        )
                } else {
                    Section::new()
                };
                rect()
                    .width(Size::fill())
                    .direction(Direction::Vertical)
                    .spacing(SPACE_2)
                    .child(text(
                        if adding {
                            tr!("new-account")
                        } else {
                            tr!("change-password")
                        },
                        HEADLINE,
                        palette.label,
                    ))
                    .child(
                        Form::new().section(
                            section.row(
                                Row::new(tr!("password")).control(
                                    rect()
                                        .direction(Direction::Horizontal)
                                        .cross_align(Alignment::Center)
                                        .spacing(SPACE_1)
                                        .child(
                                            SecureField::new(secret)
                                                .width(PASSWORD_MANAGER.password_field),
                                        )
                                        .child(
                                            Button::new(tr!("generate-password"))
                                                .on_press(generate),
                                        ),
                                ),
                            ),
                        ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .direction(Direction::Horizontal)
                            .main_align(Alignment::End)
                            .spacing(SPACE_2)
                            .child(
                                Button::new(tr!("close-cancel"))
                                    .on_press(move |_| mode.set(Mode::List)),
                            )
                            .child(
                                Button::new(tr!("save-confirm"))
                                    .style(ButtonStyle::Primary)
                                    .on_press({
                                        let current = current.clone();
                                        let mut reload = reload.clone();
                                        move |_| {
                                            let Some(typed) = secret.write().take() else {
                                                return;
                                            };
                                            secret.set(PasswordField::default());
                                            let result =
                                                Secrets::open().and_then(
                                                    |secrets| match &current {
                                                        Some(account) if !adding => secrets
                                                            .set_secret(
                                                                &account.item,
                                                                typed.as_bytes(),
                                                            ),
                                                        _ => {
                                                            let (name, user) = (
                                                                name.peek().trim().to_string(),
                                                                user.peek().trim().to_string(),
                                                            );
                                                            secrets.create(
                                                                &name,
                                                                &attributes(&name, &user),
                                                                typed.as_bytes(),
                                                            )
                                                        }
                                                    },
                                                );
                                            match result {
                                                Ok(()) => {
                                                    mode.set(Mode::List);
                                                    reload();
                                                }
                                                Err(error) => problem.set(Some(message(error))),
                                            }
                                        }
                                    }),
                            ),
                    )
                    .into_element()
            }
            Mode::ConfirmDelete(index) => rect()
                .width(Size::fill())
                .direction(Direction::Vertical)
                .spacing(SPACE_2)
                .child(text(
                    tr!("delete-password-question"),
                    HEADLINE,
                    palette.label,
                ))
                .child(
                    rect()
                        .width(Size::fill())
                        .direction(Direction::Horizontal)
                        .main_align(Alignment::End)
                        .spacing(SPACE_2)
                        .child(
                            Button::new(tr!("close-cancel"))
                                .on_press(move |_| mode.set(Mode::List)),
                        )
                        .child(
                            Button::new(tr!("password-manager-remove"))
                                .style(ButtonStyle::Primary)
                                .on_press({
                                    let mut reload = reload.clone();
                                    move |_| {
                                        let account = accounts.peek().get(index).cloned();
                                        if let Some(account) = account {
                                            match Secrets::open()
                                                .and_then(|secrets| secrets.delete(&account.item))
                                            {
                                                Ok(()) => {
                                                    selected.set(None);
                                                    reload();
                                                }
                                                Err(error) => problem.set(Some(message(error))),
                                            }
                                        }
                                        mode.set(Mode::List);
                                    }
                                }),
                        ),
                )
                .into_element(),
        };
        let listing = *mode.read() == Mode::List;
        let on_close = self.on_close.clone();
        let on_user_name = self.on_user_name.clone();
        let on_password = self.on_password.clone();
        let on_send_return = self.on_send_return.clone();
        let on_broadcast = self.on_broadcast.clone();
        let user_name = current
            .as_ref()
            .map(|account| account.user.clone())
            .filter(|name| !name.is_empty());
        sheet(
            rect()
                .width(Size::px(PASSWORD_MANAGER.width))
                .direction(Direction::Vertical)
                .spacing(SPACE_2)
                .child(text(tr!("password-manager"), HEADLINE, palette.label))
                .child(rect().key(mode_key).width(Size::fill()).child(body))
                .maybe_child((*problem.read()).clone().map(|problem| {
                    text(problem, CAPTION, palette.secondary_label)
                        .width(Size::fill())
                        .max_lines(None)
                }))
                .maybe_child(listing.then(|| {
                    rect()
                        .width(Size::fill())
                        .direction(Direction::Horizontal)
                        .cross_align(Alignment::Center)
                        .spacing(SPACE_2)
                        .child(
                            Checkbox::new(tr!("send-return-after-password"))
                                .checked(self.send_return)
                                .on_toggle(move |on| on_send_return.call(on)),
                        )
                        .child(
                            Checkbox::new(tr!("password-manager-broadcast"))
                                .checked(self.broadcast)
                                .on_toggle(move |on| on_broadcast.call(on)),
                        )
                }))
                .maybe_child(listing.then(|| {
                    rect()
                        .width(Size::fill())
                        .direction(Direction::Horizontal)
                        .main_align(Alignment::End)
                        .spacing(SPACE_2)
                        .child(Button::new(tr!("close")).on_press(move |_| on_close.call(())))
                        .child(
                            Button::new(tr!("enter-user-name"))
                                .disabled(user_name.is_none())
                                .on_press(move |_| {
                                    if let Some(name) = &user_name {
                                        on_user_name.call(name.clone());
                                    }
                                }),
                        )
                        .child(
                            Button::new(tr!("enter-password"))
                                .style(ButtonStyle::Primary)
                                .disabled(current.is_none())
                                .on_press(move |_| {
                                    if let Some(account) = &current {
                                        on_password.call(account.clone());
                                    }
                                }),
                        )
                })),
        )
    }
}
