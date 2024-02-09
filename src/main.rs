use chrono::prelude::*;
use dark_light;
use iced::font;
use iced::widget::{
    button,
    canvas::{Cache, Frame, Geometry},
    checkbox, column, container, horizontal_space, radio, row, scrollable, slider, text,
    text_input, toggler, vertical_space,
};
use iced::{Application, Color, Command, Element, Font, Length, Pixels, Settings, Size, Theme};
use log::{info, Level};
use plotters::prelude::ChartBuilder;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use plotters_iced::plotters_backend::BackendColor;
use plotters_iced::{Chart, ChartWidget, DrawingBackend, Renderer};
use reqwest;
use serde::{Deserialize, Deserializer};
use serde_repr::Deserialize_repr;

pub fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init().expect("Initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }

    #[cfg(not(target_arch = "wasm32"))]
    tracing_subscriber::fmt::init();

    App::run(Settings {
        antialiasing: true,
        default_font: Font::with_name("Montserrat"),
        ..Default::default()
    })
}

#[derive(Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
enum SensorStatus {
    Normal = 0,
    Warmup = 1,
    Startup = 2,
    Invalid = 3,
}

impl std::fmt::Display for SensorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorStatus::Normal => write!(f, "Normal operation"),
            SensorStatus::Warmup => write!(f, "Warm-up"),
            SensorStatus::Startup => write!(f, "Initial startup"),
            SensorStatus::Invalid => write!(f, "Invlid output"),
        }
    }
}

#[derive(Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
enum QualityIndex {
    Unhealthy = 5,
    Poor = 4,
    Moderate = 3,
    Good = 2,
    Excellent = 1,
    Unknown = 0,
}

fn date_de<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    Ok(Local
        .datetime_from_str(s, "%Y-%m-%d %H:%M:%S.%f")
        .unwrap()
        .with_timezone(&Local))
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct SensorState {
    #[serde(deserialize_with = "date_de")]
    time: DateTime<Local>,
    status: SensorStatus,
    qi: QualityIndex,
    tvoc: u16,
    co2: u16,
}

#[derive(Debug)]
struct App {
    state: AppState,
    theme: ThemeType,
}

#[derive(Debug)]
enum AppState {
    Loading,
    Loaded(SensorState, CO2Chart, u16),
    Errored,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Vec<SensorState>, Error>),
    Load,
    BottomSliderChanged(u16),
    TopSliderChanged(u16),
    FontLoaded(Result<(), font::Error>),
    ThemeChanged(ThemeType),
    None,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ThemeType {
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub enum Error {
    APIError,
    OtherError,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        dbg!(error);
        Error::APIError
    }
}

async fn load() -> Result<Vec<SensorState>, Error> {
    let response: Vec<SensorState> = reqwest::Client::new()
        .get(HISTORY_URL)
        .send()
        .await?
        .json()
        .await?;
    Ok(response)
}

const HISTORY_URL: &str = "https://sienkiewiczapi.duckdns.org/co2/api/history";

#[derive(Debug)]
struct CO2Chart {
    cache: Cache,
    data: Vec<SensorState>,
    bottom: u16,
    top: u16,
    theme: ThemeType,
}

impl Chart<Message> for CO2Chart {
    type State = ();

    #[inline]
    fn draw<R: Renderer, F: Fn(&mut Frame)>(
        &self,
        renderer: &R,
        bounds: Size,
        draw_fn: F,
    ) -> Geometry {
        renderer.draw_cache(&self.cache, bounds, draw_fn)
    }
    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut builder: ChartBuilder<DB>) {
        use plotters::prelude::*;

        let plot_line_color: RGBColor = match self.theme {
            ThemeType::Light => RGBColor(100, 100, 100),
            ThemeType::Dark => RGBColor(150, 150, 150),
        };

        let series_color: RGBColor = match self.theme {
            ThemeType::Light => RGBColor(30, 50, 200),
            ThemeType::Dark => RGBColor(51, 89, 218),
        };

        let data = self.data[self.bottom as usize..=self.top as usize].to_vec();

        let mut chart = builder
            .x_label_area_size(28_i32)
            .y_label_area_size(28_i32)
            .margin(8_i32)
            .build_cartesian_2d(
                data.first().unwrap().time..data.last().unwrap().time,
                440_f32..2000_f32,
            )
            .expect("Failed to build chart");

        chart
            .configure_mesh()
            .x_label_formatter(&|date| date.format("%H:%M").to_string())
            .label_style(TextStyle {
                font: ("Montserrat", 12).into_font(),
                color: BackendColor {
                    alpha: 1.,
                    rgb: match self.theme {
                        ThemeType::Light => (0, 0, 0),
                        ThemeType::Dark => (255, 255, 255),
                    },
                },
                pos: Pos::new(HPos::Left, VPos::Top),
            })
            .light_line_style(plot_line_color.mix(0.1))
            .bold_line_style(plot_line_color.mix(0.25))
            .axis_style(plot_line_color)
            .draw();
        chart
            .draw_series(LineSeries::new(
                data.iter()
                    .map(|sensor_state| (sensor_state.time, sensor_state.co2 as f32)),
                series_color,
            ))
            .expect("Failed to draw data");
    }
}

impl CO2Chart {
    fn view(&self) -> Element<Message> {
        ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn theme(&self) -> Self::Theme {
        match self.theme {
            ThemeType::Dark => Theme::Dark,
            ThemeType::Light => Theme::Light,
        }
    }

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let theme = match dark_light::detect() {
            dark_light::Mode::Light => ThemeType::Light,
            _ => ThemeType::Dark,
        };

        #[cfg(target_arch = "wasm32")]
        {
            use iced_native;
            use web_sys;
            let window = web_sys::window().unwrap();

            let (width, height) = (
                (window.inner_width().unwrap().as_f64().unwrap()) as u32,
                (window.inner_height().unwrap().as_f64().unwrap()) as u32,
            );

            iced_native::Command::single(iced_native::command::Action::Window(
                iced_native::window::Action::<App>::Resize { width, height },
            ));
            info!("Win size: {width} x {height}");
        }

        (
            App {
                state: AppState::Loading,
                theme,
            },
            Command::batch([
                font::load(include_bytes!("../Montserrat-Regular.ttf").as_slice())
                    .map(Message::FontLoaded),
                font::load(include_bytes!("../Montserrat-Bold.ttf").as_slice())
                    .map(Message::FontLoaded),
                Command::perform(load(), Message::Loaded),
            ]),
        )
    }

    fn title(&self) -> String {
        let subtitle = match self.state {
            AppState::Loading => "Loading - ",
            AppState::Loaded(_, _, _) => "",
            AppState::Errored { .. } => "Error - ",
        };

        format!("{subtitle}CO2")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::Loaded(Ok(data)) => {
                let last = data.last().unwrap().clone();
                let data_len = data.len() as u16;
                let bottom_val = match data_len >= 60 {
                    true => data_len - 60,
                    false => 0,
                };
                self.state = AppState::Loaded(
                    last,
                    CO2Chart {
                        cache: Cache::new(),
                        data,
                        bottom: bottom_val,
                        top: data_len - 1,
                        theme: self.theme,
                    },
                    data_len,
                );
                Command::none()
            }
            Message::Loaded(Err(_error)) => {
                self.state = AppState::Errored;
                Command::none()
            }
            Message::Load => match self.state {
                AppState::Loading => Command::none(),
                _ => {
                    self.state = AppState::Loading;
                    Command::perform(load(), Message::Loaded)
                }
            },
            Message::BottomSliderChanged(val) => match &self.state {
                AppState::Loaded(last, old_chart, data_len) => {
                    self.state = AppState::Loaded(
                        *last,
                        CO2Chart {
                            cache: Cache::new(),
                            data: old_chart.data.to_vec(),
                            bottom: val,
                            top: old_chart.top,
                            theme: self.theme,
                        },
                        *data_len,
                    );
                    Command::none()
                }
                _ => Command::none(),
            },

            Message::TopSliderChanged(val) => match &self.state {
                AppState::Loaded(last, old_chart, data_len) => {
                    self.state = AppState::Loaded(
                        *last,
                        CO2Chart {
                            cache: Cache::new(),
                            data: old_chart.data.to_vec(),
                            bottom: old_chart.bottom,
                            top: val,
                            theme: self.theme,
                        },
                        *data_len,
                    );
                    Command::none()
                }
                _ => Command::none(),
            },
            Message::FontLoaded(_) => Command::none(),
            Message::ThemeChanged(theme) => {
                self.theme = theme;
                match &mut self.state {
                    AppState::Loaded(_, chart, _) => {
                        chart.theme = theme;
                        chart.cache.clear();
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::None => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let choose_theme = [ThemeType::Light, ThemeType::Dark].iter().fold(
            column![text("Choose a theme:")].spacing(8),
            |column, theme| {
                column.push(radio(
                    format!("{theme:?}"),
                    *theme,
                    Some(self.theme),
                    Message::ThemeChanged,
                ))
            },
        );
        let content = match &self.state {
            AppState::Loading => column![text("Loading...")],
            AppState::Loaded(state, chart, data_len) => {
                let text_column = {
                    let bold_font = Font {
                        family: iced::font::Family::Name("Montserrat"),
                        weight: iced::font::Weight::Bold,
                        stretch: iced::font::Stretch::Normal,
                        monospaced: false,
                    };
                    column![
                        row![text("Co2:").font(bold_font), text(state.co2)].spacing(8),
                        row![text("TVOC:").font(bold_font), text(state.tvoc)].spacing(8),
                        row![
                            text("Quality index:").font(bold_font),
                            text(format!("{:?}", state.qi))
                        ]
                        .spacing(8),
                        row![
                            text("Status:").font(bold_font),
                            text(state.status.to_string())
                        ]
                        .spacing(8),
                        row![
                            text("Time updated:").size(12),
                            text(state.time.format("%m-%d %H:%M:%S")).size(12)
                        ]
                        .spacing(8)
                    ]
                };
                let sliders = column![
                    row![
                        container(slider(
                            0..=(chart.top - 2),
                            chart.bottom,
                            Message::BottomSliderChanged
                        ))
                        .width(250),
                        text(chart.data[chart.bottom as usize].time.format("%H:%M"))
                    ],
                    row![
                        container(slider(
                            (chart.bottom + 2)..=(*data_len - 1),
                            chart.top,
                            Message::TopSliderChanged
                        ))
                        .width(250),
                        text(chart.data[chart.top as usize].time.format("%H:%M"))
                    ]
                ];
                column![
                    row![text_column, sliders, choose_theme].spacing(8),
                    button("Refresh").on_press(Message::Load),
                    chart.view()
                ]
                .spacing(16)
            }
            AppState::Errored => {
                column![text("Error!"), button("Refresh").on_press(Message::Load),]
            }
        };
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .into()
    }
}
