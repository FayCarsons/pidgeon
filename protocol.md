# Druid-Crow Communication Protocol Analysis

This document provides a comprehensive analysis of the communication protocol used between the Druid REPL/CLI client and the Crow device, based on the Python codebase at `/Users/faycarsons/Desktop/Code/Python/druid`.

## Overview

The Druid-Crow system implements a dual-protocol communication architecture:
1. **Primary Protocol**: Serial communication over USB CDC for runtime operations
2. **Secondary Protocol**: USB DFU (Device Firmware Update) for firmware management

## Device Identification

### USB Identification
- **Vendor ID**: `0x0483` (STMicroelectronics)
- **Product ID**: `0x5740` (Runtime mode), `0xDF11` (DFU bootloader mode)
- **Device Description**: "crow: telephone line" 
- **Hardware ID Pattern**: `USB VID:PID=0483:5740`

### Detection Logic
```python
def find_serial_port(hwid):
    for portinfo in serial.tools.list_ports.comports():
        if hwid in portinfo.hwid:  # Must match VID:PID pair
            if os.name == "nt":     # Windows doesn't know device name
                return portinfo
            if "crow: telephone line" in portinfo.product:  # Precise detection for linux/macos
                return portinfo
    raise DeviceNotFoundError("can't find crow device")
```

## Primary Communication Protocol (Serial/USB CDC)

### Connection Parameters
- **Baudrate**: 115,200 bps
- **Timeout**: 0.1 seconds (100ms)
- **Encoding**: UTF-8
- **Line Endings**: `\r\n` for commands, `\n\r` for device responses

### Message Structure

#### 1. Standard Commands/Data
Regular Lua commands and data are sent as plain text terminated with `\r\n`:
```
command_text\r\n
```

#### 2. Control Commands
Special control commands use the prefix `^^` followed by a command character:

| Command | Purpose | Format |
|---------|---------|---------|
| `^^s` | Start script upload mode | `^^s` |
| `^^e` | Execute uploaded script | `^^e` |
| `^^w` | Write script to flash memory | `^^w` |
| `^^p` | Print current user script | `^^p` |
| `^^v` | Get firmware version | `^^v` |
| `^^b` | Enter bootloader mode | `^^b` |

#### 3. Device Events
The device sends structured events using the `^^` prefix:
```
^^event_name(arg1,arg2,...)
```

Common events:
- `^^stream(channel,value)` - Streaming data from input channels
- `^^change(channel,value)` - Change notifications from input channels

### Sending Messages

#### Basic Text (`Crow.write()`)
```python
def write(self, s):
    self.writebin(s.encode('utf-8'))

def writebin(self, b):
    if len(b) % 64 == 0:    # USB packet boundary handling
        b += b'\n'
    logger.debug(f'-> {b}')
    self.serial.write(b)
```

#### Commands with Line Endings (`Crow.writeline()`)
```python
def writeline(self, line):
    self.write(line + '\r\n')
```

#### File Upload (`Crow.writefile()`)
```python
def writefile(self, fname):
    with open(fname) as f:
        for line in f.readlines():
            self.writeline(line.rstrip())
            time.sleep(0.001)  # Small delay between lines
```

#### Script Operations
- **Execute (run without storing)**: `^^s` → file contents → `^^e`
- **Upload (store in flash)**: `^^s` → file contents → `^^w`

### Receiving Messages

#### Reading Process (`Crow.read_forever()`)
```python
async def read_forever(self):
    while True:
        sleeptime = 0.001
        try:
            r = self.read(10000)  # Read up to 10KB
            if len(r) > 0:
                lines = r.split('\n\r')  # Split on device line endings
                for line in lines:
                    self.process_line(line)
        except Exception as exc:
            if self.is_connected:
                logger.error(f'lost connection: {exc}')
            sleeptime = 0.1
            self.reconnect()
        await asyncio.sleep(sleeptime)
```

#### Message Processing (`Crow.process_line()`)
```python
def process_line(self, line):
    if "^^" in line:
        cmds = line.split('^^')
        for cmd in cmds:
            t3 = cmd.rstrip().partition('(')
            if not any(t3):
                continue
            evt = t3[0]  # Event name
            args = t3[2].rstrip(')').split(',')  # Arguments
            self.raise_event('crow_event', line, evt, args)
    elif len(line) > 0:
        self.raise_event('crow_output', line)
```

### Response Types

#### 1. Regular Output
Any text that doesn't contain `^^` is treated as regular output from the device (print statements, return values, etc.).

#### 2. Structured Events
Messages containing `^^` are parsed as events:
- **Format**: `^^event_name(arg1,arg2,...)`
- **Examples**:
  - `^^stream(1,3.14)` - Input channel 1 streaming value 3.14
  - `^^change(2,0)` - Input channel 2 changed to 0

## Error Handling and Differentiation

### Connection Errors
1. **Device Not Found**: `DeviceNotFoundError` with message "can't find crow device"
2. **Serial Port Issues**: `DeviceNotFoundError` with message "can't open serial port"
3. **Lost Connection**: Automatic reconnection with `connect_err` event

### Protocol-Level Error Detection
- **No explicit error response format** in the serial protocol
- **Timeouts**: 0.1 second read timeout indicates potential issues
- **Connection monitoring**: Exceptions during read operations trigger reconnection
- **Event-based error reporting**: Through the event handler system

### Error vs Normal Value Differentiation
1. **Structured Events** (`^^event_name(...)`) vs **Plain Output** (everything else)
2. **Connection Status**: Tracked via `is_connected` flag and connection events
3. **Exception-based**: Network/serial errors raise Python exceptions
4. **Logging**: All communication logged for debugging (`druid.log`)

## Secondary Protocol (DFU - Device Firmware Update)

### DFU Protocol Details
- **USB Class**: DFU (Device Firmware Update)
- **Interface**: 0
- **Timeout**: 5000ms (5 seconds)
- **Vendor/Product ID**: `0x0483:0xDF11` (STM32 DFU mode)

### DFU Commands
| Command | Value | Purpose |
|---------|-------|---------|
| DFU_DETACH | 0 | Detach from runtime mode |
| DFU_DNLOAD | 1 | Download data to device |
| DFU_UPLOAD | 2 | Upload data from device |
| DFU_GETSTATUS | 3 | Get operation status |
| DFU_CLRSTATUS | 4 | Clear error status |
| DFU_GETSTATE | 5 | Get current state |
| DFU_ABORT | 6 | Abort operation |

### DFU State Machine
```
DFU_STATE_APP_IDLE (0x00) → DFU_STATE_APP_DETACH (0x01)
DFU_STATE_DFU_IDLE (0x02) ↔ DFU_STATE_DFU_DOWNLOAD_* (0x03-0x05)
DFU_STATE_DFU_ERROR (0x0A) - Error state requiring reset
```

### Error Handling in DFU Mode
- **Status Checking**: Every operation followed by status verification
- **Error Recovery**: `clr_status()` function clears error conditions
- **Exception Throwing**: Failed operations raise descriptive exceptions
- **State Validation**: Operations verify expected state transitions

## WebSocket Extension

### WebSocket Server (`DruidServer`)
- **Host**: localhost
- **Port**: 6666
- **Protocol**: WebSocket over TCP
- **Purpose**: Remote control interface

### WebSocket Message Flow
1. **Incoming**: WebSocket messages → `crow.writeline()` → Serial device
2. **Outgoing**: Device output → WebSocket clients via `crow_output` event handler

## Event System Architecture

### Event Types
- `connect` - Device connected successfully
- `connect_err` - Connection failed (with exception details)
- `disconnect` - Device disconnected
- `running` - Script execution started
- `uploading` - Script upload started  
- `crow_event` - Structured device event received
- `crow_output` - Plain text output received

### Event Handler Registration
```python
handlers = {
    'connect': [lambda: self.output(' <crow connected>\n')],
    'connect_err': [lambda exc: self.output(' <crow disconnected>\n')],
    'crow_event': [self.crow_event],
    'crow_output': [lambda output: self.output(output + '\n')],
}
crow.replace_handlers(handlers)
```

## Key Protocol Characteristics

1. **Asynchronous**: Uses asyncio for non-blocking I/O
2. **Resilient**: Automatic reconnection on connection loss
3. **Line-oriented**: Commands and responses are line-based
4. **Event-driven**: Extensible event handler system
5. **Dual-mode**: Runtime (serial) and firmware update (DFU) protocols
6. **UTF-8 encoded**: All text communication uses UTF-8
7. **USB packet aware**: Handles 64-byte USB packet boundaries
8. **Timeout-based**: Short timeouts for responsiveness

## Implementation Recommendations

When implementing a compatible client:

1. **Use USB CDC serial communication** at 115,200 baud
2. **Implement proper line ending handling** (`\r\n` for commands, `\n\r` for responses)
3. **Parse `^^` prefixed messages** as structured events
4. **Handle USB packet boundaries** (pad to avoid 64-byte exact packets)
5. **Implement automatic reconnection** with exponential backoff
6. **Use async I/O** for responsive user interface
7. **Log all communication** for debugging purposes
8. **Support both runtime and DFU modes** for complete functionality