use chrono::{DateTime, Utc};
use downcast_rs::{impl_downcast, Downcast};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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

    /// Take the saved messages out of the queue, for consoles that save messages.
    fn take_messages(&mut self) -> Vec<ConsoleMessage> {
        Vec::new()
    }

    fn clone_settings(&self) -> Box<dyn Console>;
}
impl_downcast!(Console);

#[derive(
    Copy, Clone, Debug, JsonSchema, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize,
)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub time: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ConsoleLimit {
    /// The total limit of messages, in bytes, to retain.
    total: usize,
    /// If the logged content exceeds `total`, retain up to this number of bytes from
    /// the beginning. If `head >= total`, then no further messages will be retained.
    /// (`head` is not yet implemented.)
    head: usize,
}

impl Default for ConsoleLimit {
    fn default() -> Self {
        ConsoleLimit {
            total: usize::MAX,
            head: usize::MAX,
        }
    }
}

/// A console that stores messages for later use.
pub struct BufferConsole {
    messages: VecDeque<ConsoleMessage>,
    capacity: ConsoleLimit,
    min_level: ConsoleLevel,
    passthrough: Option<Box<dyn Console>>,
    current_size: usize,
}

impl Default for BufferConsole {
    fn default() -> Self {
        BufferConsole::new(ConsoleLevel::Info)
    }
}

impl BufferConsole {
    pub fn new(min_level: ConsoleLevel) -> Self {
        BufferConsole {
            capacity: ConsoleLimit::default(),
            min_level,
            messages: VecDeque::new(),
            current_size: 0,
            passthrough: None,
        }
    }

    /// Set a maximum capacity on the buffer.
    pub fn capacity(mut self, capacity: Option<ConsoleLimit>) -> Self {
        self.capacity = capacity.unwrap_or_default();
        self
    }

    /// Another console instance to which messages should be passed. This could be useful
    /// if you want to buffer the messages but also print them to another console.
    pub fn passthrough(mut self, passthrough: Option<Box<dyn Console>>) -> Self {
        self.passthrough = passthrough;
        self
    }
}

impl Console for BufferConsole {
    fn add(&mut self, message: ConsoleMessage) -> bool {
        if message.level < self.min_level {
            return false;
        }

        let message_size = message.message.len();
        while self.current_size + message_size > self.capacity.total && !self.messages.is_empty() {
            let popped_size = self
                .messages
                .pop_front()
                .map(|m| m.message.len())
                .unwrap_or(0);
            self.current_size -= popped_size;
        }

        self.current_size += message_size;
        self.messages.push_back(message);
        true
    }

    fn take_messages(&mut self) -> Vec<ConsoleMessage> {
        self.current_size = 0;
        let messages = std::mem::take(&mut self.messages);
        Vec::from(messages)
    }

    fn clone_settings(&self) -> Box<dyn Console> {
        Box::new(BufferConsole {
            capacity: self.capacity.clone(),
            min_level: self.min_level,
            messages: VecDeque::new(),
            current_size: 0,
            passthrough: self.passthrough.as_ref().map(|p| p.clone_settings()),
        })
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

    fn clone_settings(&self) -> Box<dyn Console> {
        Box::new(NullConsole {})
    }
}

/// A console implementation that just prints the messages to stdout or stderr.
pub struct PrintConsole {
    pub level: ConsoleLevel,
}

impl PrintConsole {
    pub fn new(level: ConsoleLevel) -> Self {
        PrintConsole { level }
    }
}

impl Console for PrintConsole {
    fn add(&mut self, message: ConsoleMessage) -> bool {
        if message.level < self.level {
            return false;
        }

        match message.level {
            ConsoleLevel::Debug | ConsoleLevel::Warn | ConsoleLevel::Error => {
                eprintln!("{}", message.message);
            }
            ConsoleLevel::Info => println!("{}", message.message),
        };

        true
    }

    fn clone_settings(&self) -> Box<dyn Console> {
        Box::new(PrintConsole { level: self.level })
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

        fn clone_settings(&self) -> Box<dyn Console> {
            Box::new(LogConsole {})
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

        fn clone_settings(&self) -> Box<dyn Console> {
            Box::new(SlogConsole {
                logger: self.logger.clone(),
            })
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

        fn clone_settings(&self) -> Box<dyn Console> {
            Box::new(TracingConsole {})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_print_console() {
        let mut c = PrintConsole::new(ConsoleLevel::Info);
        c.add(ConsoleMessage {
            level: ConsoleLevel::Info,
            message: String::from("test message\n"),
            time: chrono::Utc::now(),
        });

        c.add(ConsoleMessage {
            level: ConsoleLevel::Debug,
            message: String::from("debug message should not appear\n"),
            time: chrono::Utc::now(),
        });
    }
}
