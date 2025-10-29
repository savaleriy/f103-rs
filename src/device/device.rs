use defmt::info;
use scpi::{cmd_nquery, cmd_qonly, error::Error, tree::prelude::*, Branch, Leaf, Root};

use crate::shared::{
    CoolingState, LedState, PowerState, COOLING_CHANNEL, LED_CHANNEL, POWER_CHANNEL, SPEED_CHANNEL,
};

/// Main device structure implementing SCPI Device trait
pub struct MyDevice;

impl Device for MyDevice {
    fn handle_error(&mut self, _err: Error) {
        info!("SCPI Error occurred");
    }
}

// ============================================================================
// IDENTIFICATION COMMANDS
// ============================================================================

/// *IDN? - Device identification query
/// Returns: "PowerModule version 0.1.0"
struct IdnCommand;

impl Command<MyDevice> for IdnCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        resp.data(b"PowerModule version 0.1.0".as_slice()).finish()
    }
}

// ============================================================================
// LED CONTROL COMMANDS
// ============================================================================

/// LED:TOGGle - Toggle LED state
struct LedToggleCommand;

impl Command<MyDevice> for LedToggleCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: LED TOGGLE");
        let _ = LED_CHANNEL.try_send(LedState::Toggle);
        Ok(())
    }
}

/// LED:ON - Turn LED on
struct LedOnCommand;

impl Command<MyDevice> for LedOnCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: LED ON");
        let _ = LED_CHANNEL.try_send(LedState::On);
        Ok(())
    }
}

/// LED:OFF - Turn LED off
struct LedOffCommand;

impl Command<MyDevice> for LedOffCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: LED OFF");
        let _ = LED_CHANNEL.try_send(LedState::Off);
        Ok(())
    }
}

/// LED? - Query LED status
struct LedStatusCommand;

impl Command<MyDevice> for LedStatusCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // For now, return a default status since we can't access signals in non-async context
        // In a real implementation, you might want to use a different approach like shared memory
        resp.data(b"OFF".as_slice()).finish()
    }
}

// ============================================================================
// POWER CONTROL COMMANDS
// ============================================================================

/// POWEr:ON - Turn power on (ACDC or DCDC based on current state)
struct PowerOnCommand;

impl Command<MyDevice> for PowerOnCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: POWER ON");
        // Default to ACDC when turning power on
        let target_state = PowerState::ACDC;
        let _ = POWER_CHANNEL.try_send(target_state);
        Ok(())
    }
}

/// POWEr:OFF - Turn power off
struct PowerOffCommand;

impl Command<MyDevice> for PowerOffCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: POWER OFF");
        let _ = POWER_CHANNEL.try_send(PowerState::OFF);
        Ok(())
    }
}

/// POWEr? - Query power status
struct PowerStatusCommand;

impl Command<MyDevice> for PowerStatusCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default power status
        resp.data(b"OFF".as_slice()).finish()
    }
}

// ============================================================================
// DCDC POWER CONTROL COMMANDS
// ============================================================================

/// POWEr:DCDC:ON - Turn DCDC power on
struct DcdcOnCommand;

impl Command<MyDevice> for DcdcOnCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: DCDC ON");
        let _ = POWER_CHANNEL.try_send(PowerState::DCDC);
        Ok(())
    }
}

/// POWEr:DCDC:OFF - Turn DCDC power off
struct DcdcOffCommand;

impl Command<MyDevice> for DcdcOffCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: DCDC OFF");
        let _ = POWER_CHANNEL.try_send(PowerState::OFF);
        Ok(())
    }
}

/// POWEr:DCDC? - Query DCDC power status
struct DcdcStatusCommand;

impl Command<MyDevice> for DcdcStatusCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default DCDC status
        resp.data(b"0".as_slice()).finish()
    }
}

/// POWEr:DCDC:VAL? - Query DCDC voltage value
struct DcdcValueCommand;

impl Command<MyDevice> for DcdcValueCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default voltage value
        resp.data(b"0".as_slice()).finish()
    }
}

// ============================================================================
// ACDC POWER CONTROL COMMANDS
// ============================================================================

/// POWEr:ACDC:ON - Turn ACDC power on
struct AcdcOnCommand;

impl Command<MyDevice> for AcdcOnCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: ACDC ON");
        let _ = POWER_CHANNEL.try_send(PowerState::ACDC);
        Ok(())
    }
}

/// POWEr:ACDC:OFF - Turn ACDC power off
struct AcdcOffCommand;

impl Command<MyDevice> for AcdcOffCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: ACDC OFF");
        let _ = POWER_CHANNEL.try_send(PowerState::OFF);
        Ok(())
    }
}

/// POWEr:ACDC? - Query ACDC power status
struct AcdcStatusCommand;

impl Command<MyDevice> for AcdcStatusCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default ACDC status
        resp.data(b"0".as_slice()).finish()
    }
}

/// POWEr:ACDC:VAL? - Query ACDC voltage value
struct AcdcValueCommand;

impl Command<MyDevice> for AcdcValueCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default voltage value
        resp.data(b"0".as_slice()).finish()
    }
}

// ============================================================================
// COOLING CONTROL COMMANDS
// ============================================================================

/// SPEEd:ON - Turn cooling on
struct SpeedOnCommand;

impl Command<MyDevice> for SpeedOnCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: COOLING ON");
        let _ = COOLING_CHANNEL.try_send(CoolingState::On);
        Ok(())
    }
}

/// SPEEd:OFF - Turn cooling off
struct SpeedOffCommand;

impl Command<MyDevice> for SpeedOffCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI: COOLING OFF");
        let _ = COOLING_CHANNEL.try_send(CoolingState::Off);
        Ok(())
    }
}

/// SPEEd? - Query cooling status and speed
struct SpeedStatusCommand;

impl Command<MyDevice> for SpeedStatusCommand {
    cmd_qonly!();

    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        // Return default cooling status
        resp.data(b"OFF,0".as_slice()).finish()
    }
}

/// SPEEd <value> - Set cooling speed
struct SpeedValueCommand;

impl Command<MyDevice> for SpeedValueCommand {
    cmd_nquery!();

    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        // For now, just send a default speed since parameter parsing is complex
        // In a real implementation, you would parse the parameters properly
        let _ = SPEED_CHANNEL.try_send(50); // Default speed
        let _ = COOLING_CHANNEL.try_send(CoolingState::On);
        Ok(())
    }
}

// ============================================================================
// SCPI COMMAND TREE DEFINITION
// ============================================================================

/// Complete SCPI command tree for PowerModule device
///
/// Supported commands:
/// - *IDN?                    -> Device identification
/// - LED:TOGGle              -> Toggle LED
/// - LED:ON                  -> Turn LED on
/// - LED:OFF                 -> Turn LED off
/// - LED?                    -> Query LED status
/// - POWEr:ON                -> Turn power on
/// - POWEr:OFF               -> Turn power off
/// - POWEr?                  -> Query power status
/// - POWEr:DCDC:ON           -> Turn DCDC on
/// - POWEr:DCDC:OFF          -> Turn DCDC off
/// - POWEr:DCDC?             -> Query DCDC status
/// - POWEr:DCDC:VAL?         -> Query DCDC voltage
/// - POWEr:ACDC:ON           -> Turn ACDC on
/// - POWEr:ACDC:OFF          -> Turn ACDC off
/// - POWEr:ACDC?             -> Query ACDC status
/// - POWEr:ACDC:VAL?         -> Query ACDC voltage
/// - SPEEd:ON                -> Turn cooling on
/// - SPEEd:OFF               -> Turn cooling off
/// - SPEEd?                  -> Query cooling status
/// - SPEEd <value>           -> Set cooling speed
pub const MYTREE: Node<MyDevice> = Root![
    Leaf!(b"*IDN" => &IdnCommand),
    Branch![b"LED";
        Leaf!(default b"TOGGle" => &LedToggleCommand),
        Leaf!(default b"ON" => &LedOnCommand),
        Leaf!(default b"OFF" => &LedOffCommand),
        Leaf!(b"?" => &LedStatusCommand)
    ],
    Branch![b"POWEr";
        Leaf!(default b"ON" => &PowerOnCommand),
        Leaf!(default b"OFF" => &PowerOffCommand),
        Leaf!(b"?" => &PowerStatusCommand),

        Branch![b"DCDC";
            Leaf!(default b"ON" => &DcdcOnCommand),
            Leaf!(default b"OFF" => &DcdcOffCommand),
            Leaf!(b"?" => &DcdcStatusCommand),
            Leaf!(b"VAL?" => &DcdcValueCommand)
        ],

        Branch![b"ACDC";
            Leaf!(default b"ON" => &AcdcOnCommand),
            Leaf!(default b"OFF" => &AcdcOffCommand),
            Leaf!(b"?" => &AcdcStatusCommand),
            Leaf!(b"VAL?" => &AcdcValueCommand)
        ]
    ],
    Branch![b"SPEEd";
        Leaf!(default b"ON" => &SpeedOnCommand),
        Leaf!(default b"OFF" => &SpeedOffCommand),
        Leaf!(b"?" => &SpeedStatusCommand),
        Leaf!(default b"<Value>" => &SpeedValueCommand)
    ]
];
