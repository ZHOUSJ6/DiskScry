use std::ffi::{OsStr, OsString};

use crate::{
    app::AppError,
    domain::{HealthState, SmartState},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    En,
    ZhCn,
}

impl Locale {
    pub fn detect(args: &[OsString]) -> Self {
        Self::detect_from(
            args,
            std::env::var_os("LC_ALL").as_deref(),
            std::env::var_os("LANG").as_deref(),
        )
    }

    pub fn detect_from(args: &[OsString], lc_all: Option<&OsStr>, lang: Option<&OsStr>) -> Self {
        if let Some(explicit) = explicit_language(args) {
            return Self::from_tag(explicit).unwrap_or_default();
        }
        if let Some(lc_all) = lc_all {
            return lc_all.to_str().and_then(Self::from_tag).unwrap_or_default();
        }
        lang.and_then(OsStr::to_str)
            .and_then(Self::from_tag)
            .unwrap_or_default()
    }

    pub fn from_tag(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized == "en" || normalized.starts_with("en_") || normalized.starts_with("en-") {
            Some(Self::En)
        } else if normalized == "zh"
            || normalized.starts_with("zh_")
            || normalized.starts_with("zh-")
        {
            Some(Self::ZhCn)
        } else {
            None
        }
    }

    pub fn messages(self) -> &'static Messages {
        match self {
            Self::En => &EN,
            Self::ZhCn => &ZH_CN,
        }
    }

    pub fn format_error(self, error: &(dyn std::error::Error + 'static)) -> String {
        if let Some(AppError::DeviceNotFound(selector)) = error.downcast_ref::<AppError>() {
            let messages = self.messages();
            return format!(
                "{} '{}' {}",
                messages.device_not_found_prefix, selector, messages.device_not_found_suffix
            );
        }
        error.to_string()
    }

    pub fn health_label(self, health: &HealthState) -> &'static str {
        let messages = self.messages();
        match health {
            HealthState::Healthy { .. } => messages.healthy,
            HealthState::Warning { .. } => messages.warning_health,
            HealthState::Critical { .. } => messages.critical,
            HealthState::Unknown { .. } => messages.unknown,
        }
    }

    pub fn smart_label(self, smart: &SmartState) -> &'static str {
        let messages = self.messages();
        match smart {
            SmartState::Available { .. } => messages.smart_available,
            SmartState::Unavailable { .. } | SmartState::Failed { .. } => {
                messages.smart_unavailable
            }
        }
    }
}

fn explicit_language(args: &[OsString]) -> Option<&str> {
    for (index, argument) in args.iter().enumerate().skip(1) {
        let Some(value) = argument.to_str() else {
            continue;
        };
        if value == "--lang" {
            return args.get(index + 1)?.to_str();
        }
        if let Some(value) = value.strip_prefix("--lang=") {
            return Some(value);
        }
    }
    None
}

pub struct Messages {
    pub app_about: &'static str,
    pub usage_heading: &'static str,
    pub commands_heading: &'static str,
    pub options_heading: &'static str,
    pub arguments_heading: &'static str,
    pub interval_help: &'static str,
    pub language_help: &'static str,
    pub list_about: &'static str,
    pub show_about: &'static str,
    pub watch_about: &'static str,
    pub json_help: &'static str,
    pub device_help: &'static str,
    pub device: &'static str,
    pub model: &'static str,
    pub capacity: &'static str,
    pub health: &'static str,
    pub smart: &'static str,
    pub id: &'static str,
    pub protocol: &'static str,
    pub connection: &'static str,
    pub reason: &'static str,
    pub error: &'static str,
    pub warning: &'static str,
    pub action: &'static str,
    pub elevate_action: &'static str,
    pub unknown: &'static str,
    pub healthy: &'static str,
    pub warning_health: &'static str,
    pub critical: &'static str,
    pub smart_available: &'static str,
    pub smart_unavailable: &'static str,
    pub disks: &'static str,
    pub overview: &'static str,
    pub session: &'static str,
    pub no_disks: &'static str,
    pub no_temperature_samples: &'static str,
    pub temperature_celsius: &'static str,
    pub footer: &'static str,
    pub refresh_error_prefix: &'static str,
    pub help_help: &'static str,
    pub version_help: &'static str,
    pub device_not_found_prefix: &'static str,
    pub device_not_found_suffix: &'static str,
    pub smart_details: &'static str,
    pub readable_view: &'static str,
    pub raw_json_view: &'static str,
    pub temperature: &'static str,
    pub firmware: &'static str,
    pub serial: &'static str,
    pub metric: &'static str,
    pub value: &'static str,
    pub raw_value: &'static str,
    pub current: &'static str,
    pub attribute: &'static str,
    pub worst: &'static str,
    pub threshold: &'static str,
    pub interpreted: &'static str,
    pub not_available: &'static str,
    pub unknown_attribute: &'static str,
    pub ata_overall_status: &'static str,
    pub passed: &'static str,
    pub failed: &'static str,
    pub not_reported: &'static str,
    pub critical_warning: &'static str,
    pub available_spare: &'static str,
    pub available_spare_threshold: &'static str,
    pub percentage_used: &'static str,
    pub data_units_read: &'static str,
    pub data_units_written: &'static str,
    pub host_read_commands: &'static str,
    pub host_write_commands: &'static str,
    pub controller_busy_time: &'static str,
    pub power_cycles: &'static str,
    pub power_on_hours: &'static str,
    pub unsafe_shutdowns: &'static str,
    pub media_errors: &'static str,
    pub error_log_entries: &'static str,
    pub hours: &'static str,
    pub days: &'static str,
    pub minutes: &'static str,
    pub interface_not_exposed: &'static str,
    pub device_not_smart_capable: &'static str,
    pub unsupported_protocol: &'static str,
    pub unsupported_transport: &'static str,
}

const EN: Messages = Messages {
    app_about: "Read-only disk health monitor",
    usage_heading: "Usage:",
    commands_heading: "Commands:",
    options_heading: "Options:",
    arguments_heading: "Arguments:",
    interval_help: "SMART refresh interval in seconds; zero disables scheduled refresh",
    language_help: "Human interface language",
    list_about: "List every discovered physical disk",
    show_about: "Show one disk selected by emitted id or device node",
    watch_about: "Continuously refresh disk information",
    json_help: "Emit the versioned machine-readable snapshot",
    device_help: "Emitted device id, device node, or BSD name",
    device: "Device",
    model: "Model",
    capacity: "Capacity",
    health: "Health",
    smart: "SMART",
    id: "ID",
    protocol: "Protocol",
    connection: "Connection",
    reason: "Reason",
    error: "Error",
    warning: "Warning",
    action: "Action",
    elevate_action: "relaunch with administrator privileges if appropriate.",
    unknown: "Unknown",
    healthy: "Healthy",
    warning_health: "Warning",
    critical: "Critical",
    smart_available: "SMART available",
    smart_unavailable: "SMART unavailable",
    disks: "Disks",
    overview: "Overview",
    session: "Session",
    no_disks: "No physical disks discovered",
    no_temperature_samples: "No temperature samples in this session",
    temperature_celsius: "Temperature °C",
    footer: "q quit | j/k disk | Tab view | r refresh | v SMART view | PgUp/PgDn scroll | interval",
    refresh_error_prefix: "SMART unavailable",
    help_help: "Print help",
    version_help: "Print version",
    device_not_found_prefix: "device selector",
    device_not_found_suffix: "did not match an emitted device id or device node",
    smart_details: "SMART details",
    readable_view: "Readable",
    raw_json_view: "Raw JSON",
    temperature: "Temperature",
    firmware: "Firmware",
    serial: "Serial",
    metric: "Metric",
    value: "Value",
    raw_value: "Raw",
    current: "Current",
    attribute: "Attribute",
    worst: "Worst",
    threshold: "Threshold",
    interpreted: "Interpreted",
    not_available: "N/A",
    unknown_attribute: "Unknown attribute",
    ata_overall_status: "ATA overall status",
    passed: "Passed",
    failed: "Failed",
    not_reported: "Not reported",
    critical_warning: "Critical warning",
    available_spare: "Available spare",
    available_spare_threshold: "Spare threshold",
    percentage_used: "Percentage used",
    data_units_read: "Data read",
    data_units_written: "Data written",
    host_read_commands: "Host read commands",
    host_write_commands: "Host write commands",
    controller_busy_time: "Controller busy time",
    power_cycles: "Power cycles",
    power_on_hours: "Power-on time",
    unsafe_shutdowns: "Unsafe shutdowns",
    media_errors: "Media errors",
    error_log_entries: "Error log entries",
    hours: "hours",
    days: "days",
    minutes: "minutes",
    interface_not_exposed: "the device interface does not expose SMART data",
    device_not_smart_capable: "the device does not report SMART capability",
    unsupported_protocol: "unsupported protocol",
    unsupported_transport: "unsupported transport",
};

const ZH_CN: Messages = Messages {
    app_about: "只读磁盘健康监控工具",
    usage_heading: "用法：",
    commands_heading: "命令：",
    options_heading: "选项：",
    arguments_heading: "参数：",
    interval_help: "SMART 刷新间隔（秒）；设为 0 时禁用定时刷新",
    language_help: "人机界面语言",
    list_about: "列出所有已发现的物理磁盘",
    show_about: "按设备 ID 或设备节点显示单个磁盘",
    watch_about: "持续刷新磁盘信息",
    json_help: "输出带版本的机器可读快照",
    device_help: "设备 ID、设备节点或 BSD 名称",
    device: "设备",
    model: "型号",
    capacity: "容量",
    health: "健康状态",
    smart: "SMART",
    id: "ID",
    protocol: "协议",
    connection: "连接",
    reason: "原因",
    error: "错误",
    warning: "警告",
    action: "操作",
    elevate_action: "如确有需要，请使用管理员权限重新运行。",
    unknown: "未知",
    healthy: "健康",
    warning_health: "警告",
    critical: "严重",
    smart_available: "SMART 可用",
    smart_unavailable: "SMART 不可用",
    disks: "磁盘",
    overview: "概览",
    session: "本次会话",
    no_disks: "未发现物理磁盘",
    no_temperature_samples: "本次会话暂无温度样本",
    temperature_celsius: "温度 °C",
    footer: "q 退出 | j/k 硬盘 | Tab 切换 | r 刷新 | v SMART 视图 | PgUp/PgDn 滚动 | 间隔",
    refresh_error_prefix: "SMART 不可用",
    help_help: "显示帮助",
    version_help: "显示版本",
    device_not_found_prefix: "设备选择器",
    device_not_found_suffix: "未匹配任何已列出的设备 ID 或设备节点",
    smart_details: "SMART 详细信息",
    readable_view: "可读视图",
    raw_json_view: "原始 JSON",
    temperature: "温度",
    firmware: "固件",
    serial: "序列号",
    metric: "指标",
    value: "可读值",
    raw_value: "原始值",
    current: "当前值",
    attribute: "属性名称",
    worst: "最差值",
    threshold: "阈值",
    interpreted: "解释值",
    not_available: "不可用",
    unknown_attribute: "未知属性",
    ata_overall_status: "ATA 总体状态",
    passed: "通过",
    failed: "失败",
    not_reported: "未报告",
    critical_warning: "严重警告",
    available_spare: "可用备用空间",
    available_spare_threshold: "备用空间阈值",
    percentage_used: "寿命损耗",
    data_units_read: "读取数据量",
    data_units_written: "写入数据量",
    host_read_commands: "主机读取命令",
    host_write_commands: "主机写入命令",
    controller_busy_time: "控制器忙碌时间",
    power_cycles: "通电次数",
    power_on_hours: "通电时间",
    unsafe_shutdowns: "异常关机次数",
    media_errors: "介质错误",
    error_log_entries: "错误日志条目",
    hours: "小时",
    days: "天",
    minutes: "分钟",
    interface_not_exposed: "设备接口未公开 SMART 数据",
    device_not_smart_capable: "设备未报告 SMART 能力",
    unsupported_protocol: "不支持的协议",
    unsupported_transport: "不支持的传输方式",
};

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn explicit_language_overrides_environment() {
        assert_eq!(
            Locale::detect_from(
                &args(&["diskscry", "--lang", "en"]),
                Some(OsStr::new("zh_CN.UTF-8")),
                None,
            ),
            Locale::En
        );
    }

    #[test]
    fn lc_all_precedes_lang() {
        assert_eq!(
            Locale::detect_from(
                &args(&["diskscry"]),
                Some(OsStr::new("zh_CN.UTF-8")),
                Some(OsStr::new("en_US.UTF-8")),
            ),
            Locale::ZhCn
        );
    }

    #[test]
    fn unsupported_locale_defaults_to_english() {
        assert_eq!(
            Locale::detect_from(&args(&["diskscry"]), None, Some(OsStr::new("fr_FR"))),
            Locale::En
        );
    }

    #[test]
    fn lang_is_used_when_lc_all_is_absent() {
        assert_eq!(
            Locale::detect_from(&args(&["diskscry"]), None, Some(OsStr::new("zh_CN.UTF-8")),),
            Locale::ZhCn
        );
    }

    #[test]
    fn unsupported_lc_all_does_not_fall_through_to_lang() {
        assert_eq!(
            Locale::detect_from(
                &args(&["diskscry"]),
                Some(OsStr::new("C")),
                Some(OsStr::new("zh_CN.UTF-8")),
            ),
            Locale::En
        );
    }

    #[test]
    fn formats_common_application_error_in_selected_language() {
        let error = AppError::DeviceNotFound("disk99".into());
        assert_eq!(
            Locale::ZhCn.format_error(&error),
            "设备选择器 'disk99' 未匹配任何已列出的设备 ID 或设备节点"
        );
    }
}
