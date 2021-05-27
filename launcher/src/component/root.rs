use std::{
    fs::{self, read_to_string, OpenOptions},
    io,
    path::is_separator,
};

use directories::ProjectDirs;
use iced::{
    button, scrollable, text_input, Align, Button, Column, Command, Container, Length, Row,
    Scrollable, Space, Subscription, Text, TextInput,
};
use log::error;
use tokio::process;

use crate::{
    configuration::Configuration,
    icon::Icon,
    installation::Installation,
    message::Message,
    scan::scan_dir,
    server::LocalServerScanner,
    style::{DefaultTextInput, ForegroundContainer, PrimaryButton, SelectableButton},
    theme::Theme,
    world::{World, WorldInfo},
};

use super::{
    fluid_page, header, page, CreateWorldComponent, DeleteWorldComponent, ServerComponent,
    WorldComponent,
};

pub struct RootComponent {
    proj_dirs: ProjectDirs,
    configuration: Configuration,
    worlds: Vec<WorldComponent>,
    installations: Vec<Installation>,
    local_servers: Vec<ServerComponent>,
    local_server_scanner: LocalServerScanner,
    theme: Theme,
    state: RootState,
    tab: RootTab,
    visible: bool,
}

#[derive(Default)]
pub struct RootState {
    tab_servers: button::State,
    tab_worlds: button::State,
    tab_settings: button::State,
    scan_local_servers: button::State,
    worlds_scroll: scrollable::State,
    servers_scroll: scrollable::State,
    create_world: button::State,
    username: text_input::State,
}
pub enum RootTab {
    Worlds(WorldTab),
    Servers(ServerTab),
    Settings,
}

#[allow(clippy::large_enum_variant)]
pub enum WorldTab {
    List,
    Create(CreateWorldComponent),
    Delete(DeleteWorldComponent),
}

pub enum ServerTab {
    List,
    Join,
}

impl RootComponent {
    pub fn update(&mut self, message: Message) -> io::Result<Command<Message>> {
        match message {
            Message::Show => {
                self.local_servers.clear();
                self.local_server_scanner.rescan();
                self.visible = true;
            }
            Message::SelectWorldTab => self.tab = RootTab::Worlds(WorldTab::List),
            Message::SelectServerTab => {
                self.local_servers.clear();
                self.local_server_scanner.rescan();
                self.tab = RootTab::Servers(ServerTab::List)
            }
            Message::SelectSettingsTab => self.tab = RootTab::Settings,
            Message::ScanLocalServers => {
                self.local_servers.clear();
                self.local_server_scanner.rescan();
            }
            Message::SetupCreateWorld => {
                self.tab = RootTab::Worlds(WorldTab::Create(CreateWorldComponent::new(
                    &self.installations,
                )))
            }
            Message::SetupDeleteWorld(world) => {
                self.tab = RootTab::Worlds(WorldTab::Delete(DeleteWorldComponent::new(world)))
            }
            Message::PlayWorld(world) => {
                let installations: Vec<_> = self
                    .installations
                    .iter()
                    .filter(|installation| installation.supports_world(&world))
                    .collect();
                if installations.len() == 1 {
                    let installation = (*installations.first().unwrap()).clone();
                    self.visible = false;
                    let token = format!(
                        "{}#{}",
                        self.configuration.local.uuid,
                        base64::encode(&self.configuration.local.username),
                    );
                    return Ok(Command::perform(
                        async move {
                            match process::Command::new(installation.path.as_os_str())
                                .current_dir(world.path)
                                .arg("play")
                                .arg(token)
                                .spawn()
                            {
                                Ok(mut child) => match child.wait().await {
                                    Ok(_) => {}
                                    Err(error) => error!("{}", error),
                                },
                                Err(error) => error!("{}", error),
                            }
                        },
                        |_| Message::Show,
                    ));
                }
            }
            Message::JoinServer(info) => {
                let installations: Vec<_> = self
                    .installations
                    .iter()
                    .filter(|installation| installation.supports_server(&info))
                    .collect();
                if installations.len() == 1 && info.authentication.is_none() {
                    let installation = (*installations.first().unwrap()).clone();
                    self.visible = false;
                    let token = format!(
                        "{}#{}",
                        self.configuration.local.uuid,
                        base64::encode(&self.configuration.local.username),
                    );
                    return Ok(Command::perform(
                        async move {
                            match process::Command::new(installation.path.as_os_str())
                                .arg("join")
                                .arg(info.hostname)
                                .arg(info.port.to_string())
                                .arg(token)
                                .arg("--skip-verification")
                                .spawn()
                            {
                                Ok(mut child) => match child.wait().await {
                                    Ok(_) => {}
                                    Err(error) => error!("{}", error),
                                },
                                Err(error) => error!("{}", error),
                            };
                        },
                        |_| Message::Show,
                    ));
                } else {
                    self.tab = RootTab::Servers(ServerTab::Join)
                }
            }
            Message::FoundLocalServer(info) => self.local_servers.push(ServerComponent::new(info)),
            Message::CreateWorld(name, installation) => {
                let base_dir = self.proj_dirs.data_dir().join("worlds");
                let name = name.replace(is_separator, "_");
                let name = if name.is_empty() {
                    "New World".into()
                } else {
                    name
                };
                let path = if base_dir.join(&name).exists() {
                    let mut i = 2;
                    while base_dir.join(format!("{} {}", &name, i)).exists() {
                        i += 1;
                    }
                    base_dir.join(format!("{} {}", name, i))
                } else {
                    base_dir.join(name)
                };
                fs::create_dir_all(&path)?;
                let file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path.join("info.json"))?;
                let info = WorldInfo {
                    format: installation.info.format.clone(),
                };
                serde_json::to_writer_pretty(file, &info)?;
                self.worlds.push(WorldComponent::new(World {
                    path: path.clone(),
                    info,
                }));
                self.tab = RootTab::Worlds(WorldTab::List);
                self.visible = false;
                let token = format!(
                    "{}#{}",
                    self.configuration.local.uuid,
                    base64::encode(&self.configuration.local.username),
                );
                return Ok(Command::perform(
                    async move {
                        match process::Command::new(installation.path.as_os_str())
                            .current_dir(path)
                            .arg("create")
                            .arg(token)
                            .spawn()
                        {
                            Ok(mut child) => match child.wait().await {
                                Ok(_) => {}
                                Err(error) => error!("{}", error),
                            },
                            Err(error) => error!("{}", error),
                        }
                    },
                    |_| Message::Show,
                ));
            }
            Message::DeleteWorld(world) => {
                fs::remove_dir_all(world.path.clone())?;
                self.worlds.retain(|w| w.world.path != world.path);
                self.tab = RootTab::Worlds(WorldTab::List);
            }
            Message::ChangeUsername(username) => {
                self.configuration.local.username = username;
                self.save_configuration()
            }
            message => self.tab.update(message),
        }
        Ok(Command::none())
    }

    pub fn view(&mut self) -> Container<'_, Message> {
        let theme = self.theme;
        let mut column = Column::new().push(header(
            Row::new()
                .push(
                    Button::new(&mut self.state.tab_worlds, Text::new("Worlds"))
                        .on_press(Message::SelectWorldTab)
                        .style(SelectableButton(theme, self.tab.is_worlds())),
                )
                .push(
                    Button::new(&mut self.state.tab_servers, Text::new("Servers"))
                        .on_press(Message::SelectServerTab)
                        .style(SelectableButton(theme, self.tab.is_servers())),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    Button::new(
                        &mut self.state.tab_settings,
                        Row::new()
                            .push(Icon::GearFill.svg(
                                16,
                                16,
                                SelectableButton(theme, self.tab.is_settings()),
                            ))
                            .push(Text::new("Settings"))
                            .spacing(5)
                            .align_items(Align::Center),
                    )
                    .on_press(Message::SelectSettingsTab)
                    .style(SelectableButton(theme, self.tab.is_settings())),
                )
                .align_items(Align::Center)
                .width(Length::Fill)
                .spacing(10),
            theme,
        ));
        column = match &mut self.tab {
            RootTab::Worlds(tab) => match tab {
                WorldTab::List => column
                    .push(
                        Scrollable::new(&mut self.state.worlds_scroll)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(10)
                            .push(Column::with_children(
                                self.worlds
                                    .iter_mut()
                                    .map(|world| world.view(theme).into())
                                    .collect(),
                            )),
                    )
                    .push(
                        Container::new(
                            Row::new()
                                .padding(20)
                                .push(Space::with_width(Length::Fill))
                                .push(
                                    Button::new(
                                        &mut self.state.create_world,
                                        Text::new("Create world"),
                                    )
                                    .padding(5)
                                    .on_press(Message::SetupCreateWorld)
                                    .style(PrimaryButton(theme)),
                                ),
                        )
                        .style(ForegroundContainer(theme, 0.0)),
                    ),
                WorldTab::Create(component) => column.push(component.view(theme)),
                WorldTab::Delete(component) => column.push(component.view(theme)),
            },
            RootTab::Servers(tab) => match tab {
                ServerTab::List => column
                    .push(
                        Scrollable::new(&mut self.state.servers_scroll)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(10)
                            .push(Column::with_children(
                                self.local_servers
                                    .iter_mut()
                                    .map(|server| server.view(theme).into())
                                    .collect(),
                            )),
                    )
                    .push(
                        Container::new(
                            Row::new()
                                .padding(20)
                                .push(Space::with_width(Length::Fill))
                                .push(
                                    Button::new(
                                        &mut self.state.scan_local_servers,
                                        Text::new("Scan local servers"),
                                    )
                                    .padding(5)
                                    .style(PrimaryButton(theme))
                                    .on_press(Message::ScanLocalServers),
                                ),
                        )
                        .style(ForegroundContainer(theme, 0.0)),
                    ),
                ServerTab::Join => column,
            },
            RootTab::Settings => column.push(page(
                Column::new()
                    .push(Space::with_height(Length::Units(20)))
                    .push(Text::new("Local worlds & servers").size(30))
                    .push(Space::with_height(Length::Units(10)))
                    .push(Text::new("Username: "))
                    .push(
                        TextInput::new(
                            &mut self.state.username,
                            "Enter username",
                            &self.configuration.local.username,
                            Message::ChangeUsername,
                        )
                        .padding(5)
                        .style(DefaultTextInput(theme)),
                    )
                    .spacing(10),
                theme,
            )),
        };
        fluid_page(column, theme)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::from_recipe(self.local_server_scanner).map(Message::FoundLocalServer)
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn save_configuration(&self) {
        Self::save_configuration_to_dirs(&self.configuration, &self.proj_dirs)
    }

    pub fn save_configuration_to_dirs(configuration: &Configuration, proj_dirs: &ProjectDirs) {
        let contents = match toml::to_string(configuration) {
            Ok(content) => content,
            Err(error) => {
                error!("{}", error);
                return;
            }
        };
        if let Err(error) = fs::create_dir_all(proj_dirs.config_dir()) {
            error!("{}", error);
            return;
        }
        if let Err(error) = fs::write(proj_dirs.config_dir().join("config.toml"), contents) {
            error!("{}", error);
        }
    }
}

impl Default for RootComponent {
    fn default() -> Self {
        let proj_dirs =
            ProjectDirs::from("", "", "wosim").expect("could not determine home directory");
        let configuration = read_to_string(proj_dirs.config_dir().join("config.toml"))
            .ok()
            .map(|s| toml::from_str(&s).ok())
            .flatten()
            .unwrap_or_else(|| {
                let configuration = Default::default();
                Self::save_configuration_to_dirs(&configuration, &proj_dirs);
                configuration
            });
        let worlds = scan_dir(proj_dirs.data_dir().join("worlds"), |path, info| {
            WorldComponent::new(World { path, info })
        });
        let versions = Installation::scan_current_dir();
        Self {
            proj_dirs,
            configuration,
            worlds,
            installations: versions,
            theme: Theme::Dark,
            local_servers: vec![],
            local_server_scanner: LocalServerScanner::default(),
            state: RootState::default(),
            tab: RootTab::Worlds(WorldTab::List),
            visible: true,
        }
    }
}

impl RootTab {
    fn is_worlds(&self) -> bool {
        match self {
            RootTab::Worlds(_) => true,
            RootTab::Servers(_) => false,
            RootTab::Settings => false,
        }
    }

    fn is_servers(&self) -> bool {
        match self {
            RootTab::Worlds(_) => false,
            RootTab::Servers(_) => true,
            RootTab::Settings => false,
        }
    }

    fn is_settings(&self) -> bool {
        match self {
            RootTab::Worlds(_) => false,
            RootTab::Servers(_) => false,
            RootTab::Settings => true,
        }
    }

    fn update(&mut self, message: Message) {
        match self {
            RootTab::Worlds(tab) => match tab {
                WorldTab::List => panic!(),
                WorldTab::Create(component) => component.update(message),
                WorldTab::Delete(_) => panic!(),
            },
            RootTab::Servers(_) => panic!(),
            RootTab::Settings => panic!(),
        }
    }
}
