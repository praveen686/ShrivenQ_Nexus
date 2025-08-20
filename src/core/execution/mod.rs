// Execution framework for ShrivenQ
// Handles different execution modes: Backtest, Paper, Live

pub mod mode_switcher;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionMode {
    Backtest,
    Paper,
    Live,
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionMode::Backtest => write!(f, "BACKTEST"),
            ExecutionMode::Paper => write!(f, "PAPER"),
            ExecutionMode::Live => write!(f, "LIVE"),
        }
    }
}
