use crate::presentation::locale::Locale;

// Derived from CrystalDiskInfo 9.9.1 generic [Smart] language tables.
// Upstream: https://github.com/hiyohiyo/CrystalDiskInfo
// Revision: fdc8bce73ab0355c513c758ebf0f0f22662830e2
// Source files: Language/English.lang, Language/Simplified Chinese.lang
// License: MIT; see THIRD_PARTY_LICENSES.md.
pub const CRYSTAL_DISK_INFO_VERSION: &str = "9.9.1";
pub const CRYSTAL_DISK_INFO_REVISION: &str = "fdc8bce73ab0355c513c758ebf0f0f22662830e2";

struct AtaAttributeName {
    id: u8,
    en: &'static str,
    zh_cn: &'static str,
}

pub fn ata_attribute_name(id: u8, locale: Locale) -> Option<&'static str> {
    let entry = ATA_ATTRIBUTE_NAMES.iter().find(|entry| entry.id == id)?;
    Some(match locale {
        Locale::En => entry.en,
        Locale::ZhCn => entry.zh_cn,
    })
}

const ATA_ATTRIBUTE_NAMES: &[AtaAttributeName] = &[
    name(0x01, "Read Error Rate", "读取错误率"),
    name(0x02, "Throughput Performance", "吞吐性能"),
    name(0x03, "Spin-Up Time", "起转用时"),
    name(0x04, "Start/Stop Count", "启停次数"),
    name(0x05, "Reallocated Sectors Count", "重新分配扇区数"),
    name(0x06, "Read Channel Margin", "读取通道余量"),
    name(0x07, "Seek Error Rate", "寻道错误率"),
    name(0x08, "Seek Time Performance", "寻道时间性能"),
    name(0x09, "Power-On Hours", "通电时间（小时）"),
    name(0x0A, "Spin Retry Count", "起转重试次数"),
    name(0x0B, "Recalibration Retries", "重新校准重试次数"),
    name(0x0C, "Power Cycle Count", "通电次数"),
    name(0x0D, "Soft Read Error Rate stab", "软读取错误率探针"),
    name(0x16, "Current Helium Level", "目前氦气水平"),
    name(0x17, "Helium Condition Lower", "氦气状态下限"),
    name(0x18, "Helium Condition Upper", "氦气状态上限"),
    name(0x1B, "MAMR Health Monitor", "MAMR 健康监控"),
    name(0xB8, "End-to-End Error", "端到端错误"),
    name(0xBB, "Reported Uncorrectable Errors", "报告的不可校正错误"),
    name(0xBC, "Command Timeout", "命令超时"),
    name(0xBD, "High Fly Writes", "磁头非正常高度写入"),
    name(0xBE, "Airflow Temperature", "气流温度"),
    name(0xBF, "G-Sense Error Rate", "加速度感应错误率"),
    name(0xC0, "Power-off Retract Count", "断电磁头缩回计数"),
    name(0xC1, "Load/Unload Cycle Count", "磁头加载/卸载循环计数"),
    name(0xC2, "Temperature", "温度"),
    name(0xC3, "Hardware ECC recovered", "硬件 ECC 校正计数"),
    name(
        0xC4,
        "Reallocation Event Count",
        "扇区物理位置重分配事件计数(与坏道相关)",
    ),
    name(
        0xC5,
        "Current Pending Sector Count",
        "有待处置扇区数(状态存疑-需保持关注)",
    ),
    name(0xC6, "Uncorrectable Sector Count", "不可校正的扇区数"),
    name(
        0xC7,
        "UltraDMA CRC Error Count",
        "UltraDMA CRC 错误计数(与数据线或接口相关)",
    ),
    name(0xC8, "Write Error Rate", "写入错误率"),
    name(0xC9, "Soft Read Error Rate", "软读取错误率"),
    name(0xCA, "Data Address Mark Error", "数据地址标记错误"),
    name(0xCB, "Run Out Cancel", "校验和错误"),
    name(0xCC, "Soft ECC Correction", "软 ECC 校正"),
    name(0xCD, "Thermal Asperity Rate", "热骚动率(高温导致的出错)"),
    name(0xCE, "Flying Height", "磁头飞行高度"),
    name(0xCF, "Spin High Current", "起转最大电流"),
    name(
        0xD0,
        "Spin Buzz",
        "起转蜂鸣/起转阶梯(欠压启动时马达加速的流程数)",
    ),
    name(0xD1, "Offline Seek Performance", "离线寻道性能"),
    name(0xD3, "Vibration During Write", "写入期间振动(振荡)"),
    name(0xD4, "Shock During Write", "写入期间震动(冲击)"),
    name(0xDC, "Disk Shift", "盘片位移"),
    name(0xDD, "G-Sense Error Rate", "加速度感应错误率"),
    name(0xDE, "Loaded Hours", "加载所用小时数(磁头电机运转)"),
    name(0xDF, "Load/Unload Retry Count", "加载/卸载重试计数"),
    name(0xE0, "Load Friction", "加载摩擦"),
    name(0xE1, "Load/Unload Cycle Count", "加载/卸载循环计数"),
    name(0xE2, "Load 'In'-time", "磁头待命时间总计(磁头从停泊区伸出)"),
    name(0xE3, "Torque Amplification Count", "扭矩放大计数"),
    name(0xE4, "Power-Off Retract Cycle", "断电磁头缩回计数"),
    name(0xE6, "GMR Head Amplitude", "巨磁阻磁头振幅"),
    name(0xE7, "Temperature", "温度"),
    name(0xF0, "Head Flying Hours", "磁头飞行小时数"),
    name(0xF1, "Total Host Writes", "主机总计写入"),
    name(0xF2, "Total Host Reads", "主机总计读取"),
    name(0xFA, "Read Error Retry Rate", "读取错误重试率"),
    name(0xFE, "Free Fall Protection", "自由落体保护"),
];

const fn name(id: u8, en: &'static str, zh_cn: &'static str) -> AtaAttributeName {
    AtaAttributeName { id, en, zh_cn }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_catalog_is_bilingual_and_revision_pinned() {
        assert_eq!(CRYSTAL_DISK_INFO_VERSION, "9.9.1");
        assert_eq!(CRYSTAL_DISK_INFO_REVISION.len(), 40);
        assert_eq!(
            ata_attribute_name(0x05, Locale::En),
            Some("Reallocated Sectors Count")
        );
        assert_eq!(
            ata_attribute_name(0x05, Locale::ZhCn),
            Some("重新分配扇区数")
        );
        assert_eq!(ata_attribute_name(0xAA, Locale::ZhCn), None);
    }
}
