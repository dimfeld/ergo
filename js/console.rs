use chrono::{DateTime, Utc};
use downcast_rs::{impl_downcast, Downcast};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, io::Write};

#[cfg(feature = "log")]
pub use log_console::*;

#[cfg(feature = "slog")]
pub use slog_console::*;

#[cfg(feature = "tracing")]
pub use tracing_console::*;

pub trait Console: Downcast {
    /// Add a message to the buffer. Returns true if the message was retained, or
    /// false if the message was dropped. (Some console implementations may always
    /// return true.)
    fn add(&mut self, message: ConsoleMessage) -> bool;
}
impl_downcast!(Console);

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsoleLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<usize> for ConsoleLevel {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Debug,
            1 => Self::Info,
            2 => Self::Warn,
            3 => Self::Error,
            _ => Self::Debug,
        }
    }
}

pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub time: DateTime<Utc>,
    pub message: String,
}

pub struct ConsoleLimit {
    /// The total limit of messages, in bytes, to retain.
    total: usize,
    /// If the logged content exceeds `total`, retain up to this number of bytes from
    /// the beginning. If `head >= total`, then no further messages will be retained.
    /// (`head` is not yet implemented.)
    head: usize,
}

/// A console that stores messages for later use.
pub struct BufferConsole {
    pub capacity: ConsoleLimit,
    pub min_level: ConsoleLevel,
    pub messages: VecDeque<ConsoleMessage>,
    current_size: usize,
}

impl Default for BufferConsole {
    fn default() -> Self {
        BufferConsole::new(ConsoleLevel::Info, None)
    }
}

impl BufferConsole {
    pub fn new(min_level: ConsoleLevel, capacity: Option<ConsoleLimit>) -> Self {
        BufferConsole {
            capacity: capacity.unwrap_or(ConsoleLimit {
                total: usize::MAX,
                head: usize::MAX,
            }),
            min_level,
            messages: VecDeque::new(),
            current_size: 0,
        }
    }
}

impl Console for BufferConsole {
    fn add(&mut self, message: ConsoleMessage) -> bool {
        if message.level < self.min_level {
            return false;
        }

        let message_size = message.message.len();
        while self.current_size + message_size > self.capacity.total && self.messages.len() > 0 {
            let popped_size = self
                .messages
                .pop_front()
                .map(|m| m.message.len())
                .unwrap_or(0);
            self.current_size -= popped_size;
        }

        self.current_size += message_size;
        self.messages.push_back(message);
        return true;
    }
}

/// A Console implementation that ignores all messages.
pub struct NullConsole {}

impl Default for NullConsole {
    fn default() -> Self {
        NullConsole {}
    }
}

impl NullConsole {
    pub fn new() -> Self {
        NullConsole {}
    }
}

impl Console for NullConsole {
    fn add(&mut self, _message: ConsoleMessage) -> bool {
        false
    }
}

/// A console implementation that just prints the messages to stdout or stderr.
pub struct PrintConsole<STDOUT: Write, STDERR: Write> {
    pub level: ConsoleLevel,
    stdout: STDOUT,
    stderr: STDERR,
}

impl<STDOUT: Write, STDERR: Write> PrintConsole<STDOUT, STDERR> {
    pub fn new(level: ConsoleLevel, stdout: STDOUT, stderr: STDERR) -> Self {
        PrintConsole {
            level,
            stdout,
            stderr,
        }
    }

    /// Transform the PrintConsole into a new one with a difference level threshold.
    /// This is mostly useful in conjunction with default(). e.g.
    /// `PrintConsole::default().level(ConsoleLevel::Debug)`
    pub fn level(self, level: ConsoleLevel) -> Self {
        PrintConsole {
            level,
            stdout: self.stdout,
            stderr: self.stderr,
        }
    }
}

impl Default for PrintConsole<std::io::Stdout, std::io::Stderr> {
    fn default() -> Self {
        PrintConsole {
            level: ConsoleLevel::Info,
            stdout: std::io::stdout(),
            stderr: std::io::stderr(),
        }
    }
}

impl<STDOUT: Write + 'static, STDERR: Write + 'static> Console for PrintConsole<STDOUT, STDERR> {
    fn add(&mut self, message: ConsoleMessage) -> bool {
        if message.level < self.level {
            return false;
        }

        match message.level {
            ConsoleLevel::Debug | ConsoleLevel::Warn | ConsoleLevel::Error => {
                self.stderr.write_all(message.message.as_bytes()).ok()
            }
            ConsoleLevel::Info => self.stdout.write_all(message.message.as_bytes()).ok(),
        };

        return true;
    }
}

#[cfg(feature = "log")]
mod log_console {
    use super::*;

    /// A Console that sends output to the `log` crate logger
    pub struct LogConsole {}

    impl LogConsole {
        fn new() -> Self {
            LogConsole {}
        }
    }

    impl Console for LogConsole {
        fn add(&mut self, message: ConsoleMessage) -> bool {
            let level = match message.level {
                ConsoleLevel::Debug => log::Level::Debug,
                ConsoleLevel::Info => log::Level::Info,
                ConsoleLevel::Warn => log::Level::Warn,
                ConsoleLevel::Error => log::Level::Error,
            };

            ::log::log!(level, "{}", message.message);
            return true;
        }
    }
}

#[cfg(feature = "slog")]
mod slog_console {
    use super::*;

    /// A Console that sends output to a `slog` logger
    pub struct SlogConsole {
        logger: slog::Logger,
    }

    impl SlogConsole {
        pub fn new(logger: slog::Logger) -> Self {
            SlogConsole { logger }
        }
    }

    impl Console for SlogConsole {
        fn add(&mut self, message: ConsoleMessage) -> bool {
            match message.level {
                ConsoleLevel::Debug => {
                    slog::debug!(self.logger, "{}", message.message)
                }
                ConsoleLevel::Info => {
                    slog::info!(self.logger, "{}", message.message)
                }
                ConsoleLevel::Warn => {
                    slog::warn!(self.logger, "{}", message.message)
                }
                ConsoleLevel::Error => {
                    slog::error!(self.logger, "{}", message.message)
                }
            };

            return true;
        }
    }
}

#[cfg(feature = "tracing")]
mod tracing_console {
    use super::*;

    /// A Console that sends output to a `tracing` logger
    pub struct TracingConsole {}

    impl TracingConsole {
        pub fn new() -> Self {
            TracingConsole {}
        }
    }

    impl Console for TracingConsole {
        fn add(&mut self, message: ConsoleMessage) -> bool {
            match message.level {
                ConsoleLevel::Debug => {
                    tracing::event!(tracing::Level::DEBUG, "{}", message.message)
                }
                ConsoleLevel::Info => tracing::event!(tracing::Level::INFO, "{}", message.message),
                ConsoleLevel::Warn => tracing::event!(tracing::Level::WARN, "{}", message.message),
                ConsoleLevel::Error => {
                    tracing::event!(tracing::Level::ERROR, "{}", message.message)
                }
            };
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_print_console() {
        let mut c = PrintConsole::default().level(ConsoleLevel::Info);
        c.add(ConsoleMessage {
            level: ConsoleLevel::Info,
            message: String::from("test message\n"),
        });

        c.add(ConsoleMessage {
            level: ConsoleLevel::Debug,
            message: String::from("debug message should not appear\n"),
        });
    }
}
