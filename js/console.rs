use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, io::Write};

pub trait Console {
    fn add(&mut self, message: ConsoleMessage) -> bool;
}

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
    pub level: ConsoleLevel,
    pub messages: VecDeque<ConsoleMessage>,
    current_size: usize,
}

impl BufferConsole {
    pub fn new(level: ConsoleLevel, capacity: Option<ConsoleLimit>) -> Self {
        BufferConsole {
            capacity: capacity.unwrap_or(ConsoleLimit {
                total: usize::MAX,
                head: usize::MAX,
            }),
            level,
            messages: VecDeque::new(),
            current_size: 0,
        }
    }
}

impl Console for BufferConsole {
    /// Add a message to the buffer. Returns true if the message was retained, or
    /// false if the message was dropped.
    fn add(&mut self, message: ConsoleMessage) -> bool {
        if message.level < self.level {
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

impl<STDOUT: Write, STDERR: Write> Console for PrintConsole<STDOUT, STDERR> {
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
