use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, TableState, Tabs},
};

use crate::{
    app::{AppError, collect_snapshots},
    domain::DeviceSnapshot,
    platform::{DeviceEventSource, DeviceEventSubscription, DeviceInventory, SmartReader},
    presentation::{
        locale::Locale,
        output,
        smart_view::{self, Severity, SmartView, SmartViewKind},
    },
};

const SAMPLE_LIMIT: usize = 240;

pub fn run<I, R>(
    inventory: I,
    reader: R,
    interval: Duration,
    locale: Locale,
) -> Result<(), Box<dyn Error>>
where
    I: DeviceInventory + DeviceEventSource + Clone + Send + 'static,
    R: SmartReader + Clone + Send + 'static,
{
    let initial = collect_snapshots(&inventory, &reader)?;
    let subscription = inventory.subscribe()?;
    let mut state = TuiState::new(initial, interval, locale);
    let (sender, receiver) = mpsc::channel();
    let mut terminal = ratatui::init();
    let _restore_terminal = RestoreTerminal;
    run_loop(
        &mut terminal,
        &mut state,
        inventory,
        reader,
        sender,
        receiver,
        subscription,
    )
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn run_loop<I, R>(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut TuiState,
    inventory: I,
    reader: R,
    sender: Sender<Result<Vec<DeviceSnapshot>, AppError>>,
    receiver: Receiver<Result<Vec<DeviceSnapshot>, AppError>>,
    subscription: I::Subscription,
) -> Result<(), Box<dyn Error>>
where
    I: DeviceInventory + DeviceEventSource + Clone + Send + 'static,
    R: SmartReader + Clone + Send + 'static,
{
    loop {
        let mut inventory_changed = false;
        while subscription.try_recv().is_ok() {
            inventory_changed = true;
        }
        if inventory_changed && state.refresh_in_flight {
            state.refresh_pending = true;
        }
        while let Ok(result) = receiver.try_recv() {
            state.complete_refresh(result);
        }

        if inventory_changed && !state.refresh_in_flight && !state.refresh_pending {
            request_refresh(inventory.clone(), reader.clone(), sender.clone(), state);
        }

        if state.refresh_pending && !state.refresh_in_flight {
            state.refresh_pending = false;
            request_refresh(inventory.clone(), reader.clone(), sender.clone(), state);
        }

        if state.should_refresh() && !state.refresh_in_flight {
            request_refresh(inventory.clone(), reader.clone(), sender.clone(), state);
        }

        terminal.draw(|frame| render(frame, state))?;
        if event::poll(Duration::from_millis(100))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => state.select_next(),
                KeyCode::Up | KeyCode::Char('k') => state.select_previous(),
                KeyCode::Tab => state.tab = (state.tab + 1) % 3,
                KeyCode::Char('v') if state.tab == 1 => state.toggle_smart_view(),
                KeyCode::PageDown if state.tab == 1 => state.smart_page_down(),
                KeyCode::PageUp if state.tab == 1 => state.smart_page_up(),
                KeyCode::Home if state.tab == 1 => state.smart_home(),
                KeyCode::End if state.tab == 1 => state.smart_end(),
                KeyCode::Char('r') if !state.refresh_in_flight => {
                    request_refresh(inventory.clone(), reader.clone(), sender.clone(), state)
                }
                _ => {}
            }
        }
    }
}

fn request_refresh<I, R>(
    inventory: I,
    reader: R,
    sender: Sender<Result<Vec<DeviceSnapshot>, AppError>>,
    state: &mut TuiState,
) where
    I: DeviceInventory + Send + 'static,
    R: SmartReader + Send + 'static,
{
    state.refresh_in_flight = true;
    thread::spawn(move || {
        let _ = sender.send(collect_snapshots(&inventory, &reader));
    });
}

struct TuiState {
    devices: Vec<DeviceSnapshot>,
    table: TableState,
    tab: usize,
    interval: Duration,
    last_refresh: Instant,
    refresh_in_flight: bool,
    refresh_pending: bool,
    error: Option<String>,
    samples: HashMap<String, VecDeque<u64>>,
    locale: Locale,
    smart_table: TableState,
    smart_offset: usize,
    smart_page_len: usize,
    smart_content_len: usize,
    smart_raw: bool,
}

impl TuiState {
    fn new(devices: Vec<DeviceSnapshot>, interval: Duration, locale: Locale) -> Self {
        let mut state = Self {
            devices: Vec::new(),
            table: TableState::default(),
            tab: 0,
            interval,
            last_refresh: Instant::now(),
            refresh_in_flight: false,
            refresh_pending: false,
            error: None,
            samples: HashMap::new(),
            locale,
            smart_table: TableState::default(),
            smart_offset: 0,
            smart_page_len: 1,
            smart_content_len: 0,
            smart_raw: false,
        };
        state.replace_devices(devices);
        state
    }

    fn replace_devices(&mut self, devices: Vec<DeviceSnapshot>) {
        let selected_id = self.selected().map(|value| value.device.id.0.clone());
        for device in &devices {
            if let Some(temperature) = output::temperature(device) {
                let values = self.samples.entry(device.device.id.0.clone()).or_default();
                values.push_back(temperature.max(0) as u64);
                if values.len() > SAMPLE_LIMIT {
                    values.pop_front();
                }
            }
        }
        self.devices = devices;
        let index = selected_id
            .and_then(|id| {
                self.devices
                    .iter()
                    .position(|value| value.device.id.0 == id)
            })
            .or((!self.devices.is_empty()).then_some(0));
        self.table.select(index);
        self.reset_smart_viewport();
        self.last_refresh = Instant::now();
        self.error = None;
    }

    fn complete_refresh(&mut self, result: Result<Vec<DeviceSnapshot>, AppError>) {
        self.refresh_in_flight = false;
        if self.refresh_pending {
            return;
        }
        match result {
            Ok(devices) => self.replace_devices(devices),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn selected(&self) -> Option<&DeviceSnapshot> {
        self.table
            .selected()
            .and_then(|index| self.devices.get(index))
    }

    fn select_next(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        let next = self
            .table
            .selected()
            .map_or(0, |index| (index + 1) % self.devices.len());
        self.table.select(Some(next));
        self.reset_smart_viewport();
    }

    fn select_previous(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        let previous = self.table.selected().map_or(0, |index| {
            if index == 0 {
                self.devices.len() - 1
            } else {
                index - 1
            }
        });
        self.table.select(Some(previous));
        self.reset_smart_viewport();
    }

    fn should_refresh(&self) -> bool {
        !self.interval.is_zero() && self.last_refresh.elapsed() >= self.interval
    }

    fn toggle_smart_view(&mut self) {
        self.smart_raw = !self.smart_raw;
        self.reset_smart_viewport();
    }

    fn smart_page_down(&mut self) {
        let next = self.smart_offset.saturating_add(self.smart_page_len.max(1));
        self.smart_offset = next.min(self.max_smart_offset());
        self.sync_smart_table_offset();
    }

    fn smart_page_up(&mut self) {
        self.smart_offset = self.smart_offset.saturating_sub(self.smart_page_len.max(1));
        self.sync_smart_table_offset();
    }

    fn smart_home(&mut self) {
        self.smart_offset = 0;
        self.sync_smart_table_offset();
    }

    fn smart_end(&mut self) {
        self.smart_offset = self.max_smart_offset();
        self.sync_smart_table_offset();
    }

    fn set_smart_viewport(&mut self, content_len: usize, page_len: usize) {
        self.smart_content_len = content_len;
        self.smart_page_len = page_len.max(1);
        self.smart_offset = self.smart_offset.min(self.max_smart_offset());
        self.sync_smart_table_offset();
    }

    fn max_smart_offset(&self) -> usize {
        self.smart_content_len
            .saturating_sub(self.smart_page_len.max(1))
    }

    fn reset_smart_viewport(&mut self) {
        self.smart_offset = 0;
        self.smart_content_len = 0;
        self.sync_smart_table_offset();
    }

    fn sync_smart_table_offset(&mut self) {
        *self.smart_table.offset_mut() = self.smart_offset;
    }
}

fn render(frame: &mut Frame<'_>, state: &mut TuiState) {
    let messages = state.locale.messages();
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(frame.area());
    let compact_disks = state.tab == 1;
    let pane_widths = if compact_disks {
        [Constraint::Length(22), Constraint::Min(40)]
    } else {
        [Constraint::Percentage(42), Constraint::Percentage(58)]
    };
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pane_widths)
        .split(areas[0]);

    let (rows, widths, header) = if compact_disks {
        (
            state
                .devices
                .iter()
                .map(|snapshot| Row::new([snapshot.device.device_node.display().to_string()]))
                .collect::<Vec<_>>(),
            vec![Constraint::Fill(1)],
            Row::new([messages.device]),
        )
    } else {
        (
            state
                .devices
                .iter()
                .map(|snapshot| {
                    Row::new(vec![
                        Cell::from(snapshot.device.device_node.display().to_string()),
                        Cell::from(
                            snapshot
                                .device
                                .identity
                                .model
                                .clone()
                                .unwrap_or_else(|| messages.unknown.into()),
                        ),
                        Cell::from(output::health_label(&snapshot.health, state.locale)),
                        Cell::from(output::smart_label(snapshot, state.locale)),
                    ])
                })
                .collect::<Vec<_>>(),
            vec![
                Constraint::Length(12),
                Constraint::Fill(1),
                Constraint::Length(9),
                Constraint::Length(17),
            ],
            Row::new([
                messages.device,
                messages.model,
                messages.health,
                messages.smart,
            ]),
        )
    };
    let table = Table::new(rows, widths)
        .header(header.style(Style::default().add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .title(format!(" {} ", messages.disks))
                .borders(Borders::ALL),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(table, panes[0], &mut state.table);

    render_detail(frame, panes[1], state);
    let footer = state.error.as_deref().map_or_else(
        || format!("{} {}s", messages.footer, state.interval.as_secs()),
        |error| format!("{} | {error}", messages.refresh_error_prefix),
    );
    frame.render_widget(Paragraph::new(footer), areas[1]);
}

fn render_detail(frame: &mut Frame<'_>, area: ratatui::layout::Rect, state: &mut TuiState) {
    let messages = state.locale.messages();
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(area);
    frame.render_widget(
        Tabs::new([messages.overview, messages.smart, messages.session])
            .select(state.tab)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        sections[0],
    );
    let Some(selected) = state.selected().cloned() else {
        frame.render_widget(
            Paragraph::new(messages.no_disks).block(Block::bordered()),
            sections[1],
        );
        return;
    };

    match state.tab {
        0 => frame.render_widget(
            Paragraph::new(output::render_overview(&selected, state.locale)).block(
                Block::default()
                    .title(format!(" {} ", messages.overview))
                    .borders(Borders::ALL),
            ),
            sections[1],
        ),
        1 if state.smart_raw => render_smart_raw(frame, sections[1], state, &selected),
        1 => render_smart_readable(frame, sections[1], state, &selected),
        _ => {
            let values = state
                .samples
                .get(&selected.device.id.0)
                .map(VecDeque::as_slices);
            let data = values.map_or_else(Vec::new, |(first, second)| {
                first.iter().chain(second).copied().collect()
            });
            if data.is_empty() {
                frame.render_widget(
                    Paragraph::new(Line::from(messages.no_temperature_samples))
                        .block(Block::bordered()),
                    sections[1],
                );
            } else {
                frame.render_widget(
                    Sparkline::default()
                        .data(&data)
                        .block(
                            Block::default()
                                .title(format!(" {} ", messages.temperature_celsius))
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::Cyan)),
                    sections[1],
                );
            }
        }
    }
}

fn render_smart_readable(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &mut TuiState,
    snapshot: &DeviceSnapshot,
) {
    let messages = state.locale.messages();
    let view = smart_view::project_smart(snapshot, state.locale);
    let visible_diagnostics = if view.rows.is_empty() {
        0
    } else {
        view.diagnostics.len()
    };
    let summary_height = u16::try_from(view.summary.len() + visible_diagnostics)
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .min(area.height.saturating_sub(1));
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(summary_height), Constraint::Min(1)])
        .split(area);

    let mut summary = view
        .summary
        .iter()
        .map(|field| {
            Line::from(vec![
                Span::styled(
                    format!("{}: ", field.label),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(field.value.clone(), severity_style(field.severity)),
            ])
        })
        .collect::<Vec<_>>();
    if !view.rows.is_empty() {
        summary.extend(view.diagnostics.iter().map(|diagnostic| {
            Line::from(Span::styled(
                diagnostic.clone(),
                severity_style(Severity::Warning),
            ))
        }));
    }
    frame.render_widget(
        Paragraph::new(summary).block(
            Block::default()
                .title(format!(" {} ", messages.smart_details))
                .borders(Borders::ALL),
        ),
        sections[0],
    );

    if view.rows.is_empty() {
        let page_len = usize::from(sections[1].height.saturating_sub(2)).max(1);
        state.set_smart_viewport(view.diagnostics.len(), page_len);
        let text = view
            .diagnostics
            .iter()
            .skip(state.smart_offset)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .title(format!(" {} ", messages.readable_view))
                    .borders(Borders::ALL),
            ),
            sections[1],
        );
        return;
    }

    render_smart_table(frame, sections[1], state, view);
}

fn render_smart_table(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &mut TuiState,
    view: SmartView,
) {
    let messages = state.locale.messages();
    let page_len = usize::from(area.height.saturating_sub(3)).max(1);
    state.set_smart_viewport(view.rows.len(), page_len);
    let rows = view.rows.into_iter().map(|row| {
        Row::new(row.cells.into_iter().map(Cell::from).collect::<Vec<_>>())
            .style(severity_style(row.severity))
    });
    let widths = smart_table_widths(view.kind, area.width);
    let table = Table::new(rows, widths)
        .header(
            Row::new(view.columns)
                .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        )
        .block(
            Block::default()
                .title(format!(" {} ", messages.readable_view))
                .borders(Borders::ALL),
        );
    frame.render_stateful_widget(table, area, &mut state.smart_table);
}

fn smart_table_widths(kind: SmartViewKind, area_width: u16) -> Vec<Constraint> {
    match (kind, area_width) {
        (SmartViewKind::Ata, 108..) => vec![
            Constraint::Length(40),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(18),
            Constraint::Length(14),
        ],
        (SmartViewKind::Ata, _) => vec![
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
        (SmartViewKind::Nvme, 90..) => vec![
            Constraint::Length(28),
            Constraint::Length(30),
            Constraint::Length(24),
        ],
        (SmartViewKind::Nvme, _) => vec![
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Length(16),
        ],
        (SmartViewKind::Diagnostic, _) => Vec::new(),
    }
}

fn render_smart_raw(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &mut TuiState,
    snapshot: &DeviceSnapshot,
) {
    let messages = state.locale.messages();
    let text =
        serde_json::to_string_pretty(&snapshot.smart).unwrap_or_else(|error| error.to_string());
    let page_len = usize::from(area.height.saturating_sub(2)).max(1);
    state.set_smart_viewport(text.lines().count(), page_len);
    let scroll = u16::try_from(state.smart_offset).unwrap_or(u16::MAX);
    frame.render_widget(
        Paragraph::new(text).scroll((scroll, 0)).block(
            Block::default()
                .title(format!(" {} ", messages.raw_json_view))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn severity_style(severity: Severity) -> Style {
    match severity {
        Severity::Normal => Style::default(),
        Severity::Warning => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Severity::Critical => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::{Terminal, backend::TestBackend};

    use crate::domain::*;

    use super::*;

    #[derive(Clone)]
    struct EmptyInventory;

    impl DeviceInventory for EmptyInventory {
        fn list(&self) -> Result<Vec<DeviceRecord>, crate::platform::PlatformError> {
            Ok(Vec::new())
        }
    }

    #[derive(Clone)]
    struct UnusedReader;

    impl SmartReader for UnusedReader {
        fn read(&self, _device: &DeviceRecord) -> SmartState {
            unreachable!("empty inventory never requests SMART")
        }
    }

    #[test]
    fn renders_unavailable_external_disk() {
        let snapshot =
            DeviceSnapshot::unavailable(test_device(), SmartUnavailableReason::InterfaceNotExposed);
        let mut state = TuiState::new(vec![snapshot], Duration::from_secs(60), Locale::En);
        state.tab = 1;
        let rendered = render_state_at(state, 80, 40);
        assert!(rendered.contains("External Test Disk"));
        assert!(rendered.contains("SMART unavailable"));
        assert!(rendered.contains("does not expose SMART data"));
    }

    #[test]
    fn renders_failed_smart_state() {
        let snapshot = DeviceSnapshot {
            device: test_device(),
            smart: SmartState::Failed {
                error: SmartReadError {
                    stage: SmartReadStage::SmartData,
                    operation: "read SMART".into(),
                    message: "permission denied".into(),
                    native_code: Some(13),
                    permission_denied: true,
                },
            },
            health: HealthState::Unknown {
                reason: HealthUnknownReason::SmartReadFailed,
            },
            observed_at_unix_seconds: 0,
        };
        let mut state = TuiState::new(vec![snapshot], Duration::from_secs(60), Locale::En);
        state.tab = 1;
        let rendered = render_state_at(state, 80, 24);
        assert!(rendered.contains("SMART unavailable"));
        assert!(rendered.contains("permission denied"));
    }

    #[test]
    fn renders_empty_inventory() {
        let rendered = render_state(TuiState::new(
            Vec::new(),
            Duration::from_secs(60),
            Locale::En,
        ));
        assert!(rendered.contains("No physical disks discovered"));
    }

    #[test]
    fn renders_available_session_sample() {
        let nvme = test_nvme();
        let smart = SmartSnapshot::Nvme {
            data: Box::new(nvme),
        };
        let snapshot = DeviceSnapshot {
            device: test_device(),
            health: evaluate_health(&smart),
            smart: SmartState::Available {
                snapshot: Box::new(smart),
                warnings: Vec::new(),
            },
            observed_at_unix_seconds: 0,
        };
        let mut state = TuiState::new(vec![snapshot], Duration::from_secs(60), Locale::En);
        state.tab = 2;
        let rendered = render_state(state);
        assert!(rendered.contains("Temperature °C"));
    }

    #[test]
    fn smart_tab_defaults_to_readable_nvme_metrics() {
        let mut state = TuiState::new(
            vec![available_nvme_snapshot()],
            Duration::from_secs(60),
            Locale::ZhCn,
        );
        state.tab = 1;
        let rendered = render_state_at(state, 80, 40);
        assert!(rendered.contains("可读视图"));
        assert!(rendered.contains("读取数据量 (06)"));
        assert!(rendered.contains("介质错误 (0E)"));
        assert!(rendered.contains("1.0 TB"));
        assert!(rendered.contains("2000000"));
    }

    #[test]
    fn wide_smart_table_keeps_metric_and_value_together() {
        let mut state = TuiState::new(
            vec![available_nvme_snapshot()],
            Duration::from_secs(60),
            Locale::En,
        );
        state.tab = 1;
        let rendered = render_state_at(state, 220, 24);
        let line = rendered
            .lines()
            .find(|line| line.contains("Data read"))
            .unwrap();
        let metric = line.find("Data read").unwrap();
        let value = line.find("1.0 TB").unwrap();
        assert!((20..=36).contains(&(value - metric)));
    }

    #[test]
    fn smart_tab_toggles_to_raw_json() {
        let mut state = TuiState::new(
            vec![available_nvme_snapshot()],
            Duration::from_secs(60),
            Locale::En,
        );
        state.tab = 1;
        state.toggle_smart_view();
        let rendered = render_state(state);
        assert!(rendered.contains("Raw JSON"));
        assert!(rendered.contains("critical_warning"));
    }

    #[test]
    fn smart_viewport_pages_and_resets_on_disk_selection() {
        let mut second = available_nvme_snapshot();
        second.device.id = DeviceId("disk:second".into());
        let mut state = TuiState::new(
            vec![available_nvme_snapshot(), second],
            Duration::from_secs(60),
            Locale::En,
        );
        state.set_smart_viewport(30, 10);
        state.smart_page_down();
        assert_eq!(state.smart_offset, 10);
        state.smart_end();
        assert_eq!(state.smart_offset, 20);
        state.smart_page_up();
        assert_eq!(state.smart_offset, 10);
        state.select_next();
        assert_eq!(state.smart_offset, 0);
    }

    #[test]
    fn scheduler_respects_disabled_and_elapsed_intervals() {
        let disabled = TuiState::new(Vec::new(), Duration::ZERO, Locale::En);
        assert!(!disabled.should_refresh());

        let mut elapsed = TuiState::new(Vec::new(), Duration::from_secs(5), Locale::En);
        elapsed.last_refresh = Instant::now() - Duration::from_secs(6);
        assert!(elapsed.should_refresh());
    }

    #[test]
    fn manual_refresh_runs_on_worker_thread() {
        let (sender, receiver) = mpsc::channel();
        let mut state = TuiState::new(Vec::new(), Duration::ZERO, Locale::En);
        request_refresh(EmptyInventory, UnusedReader, sender, &mut state);
        assert!(state.refresh_in_flight);
        assert!(
            receiver
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn hotplug_discards_stale_in_flight_snapshot() {
        let original =
            DeviceSnapshot::unavailable(test_device(), SmartUnavailableReason::InterfaceNotExposed);
        let mut state = TuiState::new(vec![original], Duration::from_secs(60), Locale::En);
        state.refresh_in_flight = true;
        state.refresh_pending = true;
        state.complete_refresh(Ok(Vec::new()));
        assert_eq!(state.devices.len(), 1);
        assert!(!state.refresh_in_flight);
        assert!(state.refresh_pending);
    }

    #[test]
    fn renders_simplified_chinese_interface() {
        let snapshot =
            DeviceSnapshot::unavailable(test_device(), SmartUnavailableReason::InterfaceNotExposed);
        let rendered = render_state(TuiState::new(
            vec![snapshot],
            Duration::from_secs(60),
            Locale::ZhCn,
        ));
        assert!(rendered.contains("磁盘"));
        assert!(rendered.contains("健康状态"));
        assert!(rendered.contains("SMART 不可用"));
        assert!(rendered.contains("q 退出"));
    }

    fn test_device() -> DeviceRecord {
        DeviceRecord {
            id: DeviceId("disk:external".into()),
            generation: 1,
            device_node: PathBuf::from("/dev/disk9"),
            identity: DeviceIdentity {
                model: Some("External Test Disk".into()),
                serial: None,
                firmware: None,
            },
            connection: ConnectionInfo {
                protocol: StorageProtocol::Scsi,
                bus: ConnectionBus::Usb,
                removable: true,
            },
            capacity_bytes: 1_000_000_000,
            external: true,
        }
    }

    fn test_nvme() -> NvmeSmartSnapshot {
        NvmeSmartSnapshot {
            identity: DeviceIdentityData::default(),
            critical_warning: 0,
            temperature_celsius: Some(31),
            available_spare_percent: 100,
            available_spare_threshold_percent: 10,
            percentage_used: 1,
            data_units_read: DecimalCounter("2000000".into()),
            data_units_written: DecimalCounter("0".into()),
            host_read_commands: DecimalCounter("0".into()),
            host_write_commands: DecimalCounter("0".into()),
            controller_busy_minutes: DecimalCounter("0".into()),
            power_cycles: DecimalCounter("0".into()),
            power_on_hours: DecimalCounter("0".into()),
            unsafe_shutdowns: DecimalCounter("0".into()),
            media_errors: DecimalCounter("0".into()),
            error_log_entries: DecimalCounter("0".into()),
        }
    }

    fn available_nvme_snapshot() -> DeviceSnapshot {
        let smart = SmartSnapshot::Nvme {
            data: Box::new(test_nvme()),
        };
        DeviceSnapshot {
            device: test_device(),
            health: evaluate_health(&smart),
            smart: SmartState::Available {
                snapshot: Box::new(smart),
                warnings: Vec::new(),
            },
            observed_at_unix_seconds: 0,
        }
    }

    fn render_state(state: TuiState) -> String {
        render_state_at(state, 120, 24)
    }

    fn render_state_at(mut state: TuiState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut state)).unwrap();
        terminal.backend().to_string()
    }
}
