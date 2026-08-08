# Linux native SMART backend design

## Integration boundary

The Linux module implements the inventory-event and read-only SMART transport traits established by the macOS child. Shared models, protocol parsers, health evaluation, refresh scheduling, JSON, CLI, and TUI do not gain Linux conditionals.

## Discovery and events

Whole physical devices are identified from sysfs block-device topology. Partitions, device-mapper logical devices, loop devices, and optical devices are classified explicitly and are not emitted as physical disks unless the parent contract is expanded.

Kernel device events update the inventory on add, remove, and change. Each event is enriched from sysfs before entering the shared store. External classification uses bus and topology evidence and remains separate from SMART support.

## NVMe transport

The NVMe adapter opens the controller device and issues Linux NVMe admin ioctls for Identify Controller and Get Log Page. Request structures, alignment, data lengths, endian conversion, and ioctl return codes are contained in the platform module. Returned buffers enter the shared NVMe parser.

## ATA transport

The ATA adapter uses the SCSI generic `SG_IO` interface with read-only ATA PASS-THROUGH commands. It records SCSI status, host status, driver status, and sense bytes in acquisition errors. Unsupported sense data maps to `Unavailable` only when it proves the transport lacks the required pass-through; permission and malformed responses remain `Failed`.

No proprietary bridge command set or RAID-controller syntax is attempted.

## Safety and validation

All ioctl calls validate file descriptor ownership, buffer length, direction, structure layout, and returned byte count. Safe APIs expose only identify and SMART reads. Linux-specific tests exercise request construction and response/error mapping without treating test fixtures as a production transport.

Hardware validation covers native NVMe, direct ATA or SAT where available, unsupported USB bridge, permission denial, hot removal, and read completion racing removal.
