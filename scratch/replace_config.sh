#!/bin/bash
sed -i 's/pub enum Command {/#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]\npub enum SecurityMode {\n    Local,\n    Remote,\n}\n\n#[derive(Subcommand, Debug)]\npub enum Command {/' packages/rust-tools/src/relay_agent/config.rs
sed -i 's/    pub port: u16,/    pub port: u16,\n\n    #[arg(long, value_enum, env = "RELAY_AGENT_MODE", default_value_t = SecurityMode::Local)]\n    pub mode: SecurityMode,/' packages/rust-tools/src/relay_agent/config.rs
