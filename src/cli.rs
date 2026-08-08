use std::{ffi::OsString, time::Duration};

use clap::{Arg, ArgAction, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};

use crate::{
    app::{SnapshotEnvelope, collect_snapshots, select_device},
    platform::{
        DeviceEventSource, DeviceEventSubscription,
        macos::{MacOsInventory, MacOsSmartReader},
    },
    presentation::{
        locale::{Locale, Messages},
        output, tui,
    },
};

#[derive(Debug, Parser)]
#[command(name = "diskscry", version)]
pub struct Cli {
    #[arg(long, default_value_t = 60, global = true)]
    interval: u64,
    #[arg(long, value_enum, global = true)]
    lang: Option<LanguageArg>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LanguageArg {
    #[value(name = "en")]
    En,
    #[value(name = "zh-CN")]
    ZhCn,
}

impl From<LanguageArg> for Locale {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::En => Self::En,
            LanguageArg::ZhCn => Self::ZhCn,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    List {
        #[arg(long)]
        json: bool,
    },
    Show {
        device: String,
        #[arg(long)]
        json: bool,
    },
    Watch {
        device: Option<String>,
    },
}

pub fn run(cli: Cli, locale: Locale) -> Result<(), Box<dyn std::error::Error>> {
    let inventory = MacOsInventory;
    let reader = MacOsSmartReader;
    let interval = cli.interval;
    match cli.command {
        None => tui::run(inventory, reader, Duration::from_secs(interval), locale),
        Some(Command::List { json }) => {
            let devices = collect_snapshots(&inventory, &reader)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&SnapshotEnvelope::new(devices))?
                );
            } else {
                print!("{}", output::render_list(&devices, locale));
            }
            Ok(())
        }
        Some(Command::Show { device, json }) => {
            let devices = collect_snapshots(&inventory, &reader)?;
            let selected = select_device(&devices, &device)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&SnapshotEnvelope::new(vec![selected.clone()]))?
                );
            } else {
                print!("{}", output::render_detail(selected, locale));
            }
            Ok(())
        }
        Some(Command::Watch { device }) => watch(&inventory, &reader, device, interval, locale),
    }
}

fn watch(
    inventory: &MacOsInventory,
    reader: &MacOsSmartReader,
    selector: Option<String>,
    interval: u64,
    locale: Locale,
) -> Result<(), Box<dyn std::error::Error>> {
    let subscription = inventory.subscribe()?;
    loop {
        let devices = collect_snapshots(inventory, reader)?;
        if let Some(selector) = &selector {
            print!(
                "{}",
                output::render_detail(select_device(&devices, selector)?, locale)
            );
        } else {
            print!("{}", output::render_list(&devices, locale));
        }
        if interval == 0 {
            subscription.recv()?;
            coalesce_disk_events(&subscription)?;
        } else {
            match subscription.recv_timeout(Duration::from_secs(interval)) {
                Ok(_) => coalesce_disk_events(&subscription)?,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}

fn coalesce_disk_events(
    subscription: &impl DeviceEventSubscription,
) -> Result<(), std::sync::mpsc::RecvTimeoutError> {
    loop {
        match subscription.recv_timeout(Duration::from_millis(100)) {
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

pub fn parse() -> Result<(Cli, Locale), clap::Error> {
    parse_from(std::env::args_os().collect())
}

fn parse_from(args: Vec<OsString>) -> Result<(Cli, Locale), clap::Error> {
    let detected = Locale::detect(&args);
    let matches = localized_command(detected).try_get_matches_from(args)?;
    let cli = Cli::from_arg_matches(&matches)?;
    let locale = cli.lang.map(Locale::from).unwrap_or(detected);
    Ok((cli, locale))
}

fn localized_command(locale: Locale) -> clap::Command {
    let messages = locale.messages();
    localize_root(Cli::command(), messages)
        .mut_subcommand("list", |command| {
            localize_subcommand(command, locale, false)
                .about(messages.list_about)
                .mut_arg("json", |arg| arg.help(messages.json_help))
        })
        .mut_subcommand("show", |command| {
            localize_subcommand(command, locale, true)
                .about(messages.show_about)
                .mut_arg("device", |arg| arg.help(messages.device_help))
                .mut_arg("json", |arg| arg.help(messages.json_help))
        })
        .mut_subcommand("watch", |command| {
            localize_subcommand(command, locale, true)
                .about(messages.watch_about)
                .mut_arg("device", |arg| arg.help(messages.device_help))
        })
}

fn localize_root(command: clap::Command, messages: &'static Messages) -> clap::Command {
    command
        .about(messages.app_about)
        .help_template(root_help_template(messages))
        .mut_arg("interval", |arg| arg.help(messages.interval_help))
        .mut_arg("lang", |arg| arg.help(messages.language_help))
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .arg(localized_help_arg(messages).global(true))
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .help(messages.version_help)
                .action(ArgAction::Version),
        )
}

fn localize_subcommand(
    command: clap::Command,
    locale: Locale,
    has_positionals: bool,
) -> clap::Command {
    let template = if has_positionals {
        argument_help_template(locale.messages())
    } else {
        option_help_template(locale.messages())
    };
    command.help_template(template).disable_help_flag(true)
}

fn localized_help_arg(messages: &'static Messages) -> Arg {
    Arg::new("help")
        .short('h')
        .long("help")
        .help(messages.help_help)
        .action(ArgAction::Help)
}

fn root_help_template(messages: &Messages) -> String {
    format!(
        "{{about-with-newline}}\n{} {{usage}}\n\n{}\n{{subcommands}}\n\n{}\n{{options}}",
        messages.usage_heading, messages.commands_heading, messages.options_heading
    )
}

fn argument_help_template(messages: &Messages) -> String {
    format!(
        "{{about-with-newline}}\n{} {{usage}}\n\n{}\n{{positionals}}\n\n{}\n{{options}}",
        messages.usage_heading, messages.arguments_heading, messages.options_heading
    )
}

fn option_help_template(messages: &Messages) -> String {
    format!(
        "{{about-with-newline}}\n{} {{usage}}\n\n{}\n{{options}}",
        messages.usage_heading, messages.options_heading
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_command_selects_tui_defaults() {
        let (cli, locale) = parse_from(vec![OsString::from("diskscry")]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.interval, 60);
        assert_eq!(locale, Locale::En);
    }

    #[test]
    fn watch_accepts_disabled_scheduled_refresh() {
        let (cli, _) = parse_from(
            ["diskscry", "watch", "disk4", "--interval", "0"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        )
        .unwrap();
        assert_eq!(cli.interval, 0);
        assert!(matches!(
            cli.command,
            Some(Command::Watch {
                device: Some(device)
            }) if device == "disk4"
        ));
    }

    #[test]
    fn chinese_help_localizes_human_text_but_not_commands() {
        let error = parse_from(
            ["diskscry", "--lang", "zh-CN", "--help"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("只读磁盘健康监控工具"));
        assert!(rendered.contains("用法："));
        assert!(rendered.contains("列出所有已发现的物理磁盘"));
        assert!(rendered.contains("list"));
        assert!(rendered.contains("--lang <LANG>"));
    }

    #[test]
    fn chinese_subcommand_help_localizes_arguments_and_options() {
        let error = parse_from(
            ["diskscry", "--lang", "zh-CN", "show", "--help"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("参数："));
        assert!(rendered.contains("<DEVICE>"));
        assert!(rendered.contains("设备 ID、设备节点或 BSD 名称"));
        assert!(rendered.contains("--json"));
    }
}
