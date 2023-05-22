use gdbstub::common::Signal;
use gdbstub::conn::{Connection, ConnectionExt};
use gdbstub::stub::{run_blocking, DisconnectReason, GdbStub, GdbStubError};
use gdbstub::stub::SingleThreadStopReason;
use gdbstub::target::{Target, TargetResult, TargetError};
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::singlethread::{SingleThreadBase, SingleThreadResume, SingleThreadSingleStep, SingleThreadResumeOps};
use gdbstub::target::ext::breakpoints::{SwBreakpointOps, Breakpoints, SwBreakpoint, BreakpointsOps};
use nom::AsBytes;
use tokio::sync::watch::Receiver;
use std::fmt;
use std::sync::mpsc::{Sender};
use std::sync::{mpsc, Arc};
use std::rc::Rc;
use std::time::Duration;

pub struct GbaTarget {
    pub emulation_tx_channel: Sender<GbaDebugCommand>,
    pub signal_tx_channel: Sender<Signal>,
    pub state_rx_channel: Receiver<Option<SingleThreadStopReason<u32>>>,
}

pub enum GbaDebugCommandResult {
    Executed,
    Registers { regs: gdbstub_arch::arm::reg::ArmCoreRegs },
    Data { bytes: Vec<u8> },
}

pub enum GbaDebugCommand {
    ReadRegisters{ tx: Sender<GbaDebugCommandResult> },
    WriteRegisters{ tx: Sender<GbaDebugCommandResult>, regs: gdbstub_arch::arm::reg::ArmCoreRegs },
    ReadAddress{ tx: Sender<GbaDebugCommandResult>, start_address: u32, len: usize },
    WriteAddress{ tx: Sender<GbaDebugCommandResult>, start_address: u32, data: Vec<u8> },
    Resume{ tx: Sender<GbaDebugCommandResult> },
    SingleStep{ tx: Sender<GbaDebugCommandResult> },
    AddSwBreakpoint{ tx: Sender<GbaDebugCommandResult>, addr: u32, kind: gdbstub_arch::arm::ArmBreakpointKind },
    RemoveSwBreakpoint{ tx: Sender<GbaDebugCommandResult>, addr: u32, kind: gdbstub_arch::arm::ArmBreakpointKind }
}

#[derive(Debug)]
pub enum GbaTargetError {
    SystemError(Box<dyn std::error::Error>),
    UnexpectedResult,
}

impl fmt::Display for GbaTargetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GbaTargetError::SystemError(err) => {
                write!(f, "System error: {}", err)
            },
            GbaTargetError::UnexpectedResult => write!(f, "Unexpected result"),
        }
    }
}

impl Target for GbaTarget {
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = GbaTargetError;

    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    // opt-in to support for setting/removing breakpoints
    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<Self>> {
        Some(self)
    }
}

impl SingleThreadBase for GbaTarget {
    fn read_registers(
            &mut self,
            regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
        ) -> TargetResult<(), Self> {
        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::ReadRegisters { tx });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Registers { regs: result_regs } => {
                        *regs = result_regs;
                        Ok(())
                    },
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }

    fn write_registers(&mut self, regs: &gdbstub_arch::arm::reg::ArmCoreRegs)
            -> TargetResult<(), Self> {
        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::WriteRegisters { tx, regs: regs.clone() });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(()),
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }

    fn read_addrs(
            &mut self,
            start_addr: u32,
            data: &mut [u8],
        ) -> TargetResult<(), Self> {

        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::ReadAddress { tx: tx, start_address: start_addr, len: data.len() });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Data{bytes} => {
                        data.copy_from_slice(bytes.as_bytes());
                        Ok(())
                    },
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }

    fn write_addrs(
            &mut self,
            start_addr: u32,
            data: &[u8],
        ) -> TargetResult<(), Self> {
        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::WriteAddress { tx, start_address: start_addr, data: data.try_into().unwrap() });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(()),
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }

    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadResume for GbaTarget {
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        println!("resume");

        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::Resume { tx });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(()),
                    _ => Err(GbaTargetError::UnexpectedResult),
                }
            },
            Err(error) => Err(GbaTargetError::SystemError(Box::new(error)))
        }
    }

    #[inline(always)]
    fn support_single_step(&mut self) -> Option<gdbstub::target::ext::base::singlethread::SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadSingleStep for GbaTarget {
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        println!("step");

        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::SingleStep { tx });

        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(()),
                    _ => Err(GbaTargetError::UnexpectedResult),
                }
            },
            Err(error) => Err(GbaTargetError::SystemError(Box::new(error))),
        }
    }
}

impl Breakpoints for GbaTarget {
    // there are several kinds of breakpoints - this target uses software breakpoints
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        Some(self)
    }
}

impl SwBreakpoint for GbaTarget {
    fn add_sw_breakpoint(
            &mut self,
            addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
            kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
        ) -> TargetResult<bool, Self> {

        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::AddSwBreakpoint { tx, addr, kind });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(true),
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }

    fn remove_sw_breakpoint(
            &mut self,
            addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
            kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
        ) -> TargetResult<bool, Self> {

        let (tx, rx) = mpsc::channel::<GbaDebugCommandResult>();
        let _ = self.emulation_tx_channel.send(GbaDebugCommand::RemoveSwBreakpoint { tx, addr, kind });
        
        match rx.recv() {
            Ok(result) => {
                match result {
                    GbaDebugCommandResult::Executed => Ok(true),
                    _ => Err(TargetError::Fatal(GbaTargetError::UnexpectedResult)),
                }
            },
            Err(error) => Err(TargetError::Fatal(GbaTargetError::SystemError(Box::new(error))))
        }
    }
}

enum GdbEventLoop {}

impl run_blocking::BlockingEventLoop for GdbEventLoop {
    type Target = GbaTarget;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;

    type StopReason = SingleThreadStopReason<u32>;

    // Invoked immediately after the target's `resume` method has been
    // called. The implementation should block until either the target
    // reports a stop reason, or if new data was sent over the connection.
    fn wait_for_stop_reason(
                target: &mut Self::Target,
                conn: &mut Self::Connection,
            ) -> Result<
                run_blocking::Event<Self::StopReason>,
                run_blocking::WaitForStopReasonError<
                    <Self::Target as Target>::Error,
                    <Self::Connection as Connection>::Error,
                >,
            > {

        loop {
            if let Ok(Some(_)) = conn.peek() {
                let byte = conn.read().map_err(run_blocking::WaitForStopReasonError::Connection)?;

                return Ok(run_blocking::Event::IncomingData(byte));
            }

            if let Some(reason) = *target.state_rx_channel.borrow() {
                return Ok(run_blocking::Event::TargetStopped(reason));
            }

            std::thread::yield_now();
        }
    }

    fn on_interrupt(
                target: &mut Self::Target,
            ) -> Result<Option<Self::StopReason>, <Self::Target as Target>::Error> {
        println!("interrupt");

        let _ = target.signal_tx_channel.send(Signal::SIGINT);

        // Wait til the emulated stoppped
        while let None = *target.state_rx_channel.borrow() {
            std::thread::yield_now();
        }

        Ok(*target.state_rx_channel.borrow())
    }
}

pub fn gdb_event_loop_thread(debugger: GdbStub<GbaTarget, Box<dyn ConnectionExt<Error = std::io::Error>>>, mut target: GbaTarget) {
    match debugger.run_blocking::<GdbEventLoop>(&mut target) {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => {
                println!("Client disconnected")
            }
            DisconnectReason::TargetExited(code) => {
                println!("Target exited with code {}", code)
            }
            DisconnectReason::TargetTerminated(sig) => {
                println!("Target terminated with signal {}", sig)
            }
            DisconnectReason::Kill => println!("GDB sent a kill command"),
        },
        Err(GdbStubError::TargetError(e)) => {
            println!("target encountered a fatal error: {}", e)
        },
        Err(e) => {
            println!("gdbstub encountered a fatal error: {}", e)
        }
    }
}
