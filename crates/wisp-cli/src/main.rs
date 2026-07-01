//! wisp-cli: headless test harness for wisp-core + wisp-engine — lets us
//! exercise profile parsing, config generation, and the real sing-box
//! runner without a GUI (and, for `parse`/`gen`, without Windows).

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use wisp_core::{build_config, import, BuildSettings, Profile, SplitConfig, SplitMode, SplitRule};
use wisp_engine::{locate_resources, Engine, SingBoxProcess};

#[derive(Parser)]
#[command(name = "wisp", about = "Headless test harness for wisp-core + wisp-engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a profile file and print a summary of its outbounds.
    Parse {
        /// Path to a sing-box JSON config, or a file of vless://.../hysteria2://... links.
        file: PathBuf,
    },
    /// Build a sing-box config from a profile file and print it as JSON.
    Gen {
        /// Path to a sing-box JSON config, or a file of share links.
        file: PathBuf,
        /// TUN interface MTU.
        #[arg(long, default_value_t = 1280)]
        mtu: u32,
        /// Split-tunnel mode.
        #[arg(long, value_enum, default_value_t = SplitModeArg::Off)]
        mode: SplitModeArg,
        /// A split-tunnel rule, in `kind:value` form (repeatable). Kinds:
        /// process, process_path, domain_suffix, ip_cidr.
        #[arg(long = "rule")]
        rules: Vec<String>,
    },
    /// Import + build a config + run sing-box end to end, printing live
    /// traffic stats until Ctrl-C.
    Run {
        /// Path to a sing-box JSON config, or a file of share links.
        file: PathBuf,
        /// TUN interface MTU.
        #[arg(long, default_value_t = 1280)]
        mtu: u32,
        /// Split-tunnel mode.
        #[arg(long, value_enum, default_value_t = SplitModeArg::Off)]
        mode: SplitModeArg,
        /// A split-tunnel rule, in `kind:value` form (repeatable).
        #[arg(long = "rule")]
        rules: Vec<String>,
        /// Path to the sing-box binary. Defaults to auto-detection via
        /// `wisp_engine::locate_resources`.
        #[arg(long)]
        binary: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SplitModeArg {
    Off,
    Exclude,
    Include,
}

impl From<SplitModeArg> for SplitMode {
    fn from(mode: SplitModeArg) -> Self {
        match mode {
            SplitModeArg::Off => SplitMode::Off,
            SplitModeArg::Exclude => SplitMode::Exclude,
            SplitModeArg::Include => SplitMode::Include,
        }
    }
}

/// Parse a single `--rule kind:value` flag into a `SplitRule`.
fn parse_rule(spec: &str) -> Result<SplitRule> {
    let (kind, value) = spec
        .split_once(':')
        .with_context(|| format!("rule '{spec}' must be in the form kind:value"))?;
    let rule = match kind {
        "process" => SplitRule::Process(value.to_string()),
        "process_path" => SplitRule::ProcessPath(value.to_string()),
        "domain_suffix" => SplitRule::DomainSuffix(value.to_string()),
        "ip_cidr" => SplitRule::IpCidr(value.to_string()),
        other => anyhow::bail!(
            "unknown rule kind '{other}' (expected process|process_path|domain_suffix|ip_cidr)"
        ),
    };
    Ok(rule)
}

fn split_config(mode: SplitModeArg, rules: &[String]) -> Result<SplitConfig> {
    let rules = rules.iter().map(|r| parse_rule(r)).collect::<Result<Vec<_>>>()?;
    Ok(SplitConfig {
        mode: mode.into(),
        rules,
    })
}

fn read_profile(file: &Path) -> Result<Profile> {
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    import(&text).with_context(|| format!("parsing {}", file.display()))
}

fn cmd_parse(file: &Path) -> Result<()> {
    let profile = read_profile(file)?;
    let tags = profile.tags();
    println!("profile: {} ({})", profile.name, profile.id);
    println!("outbounds: {}", tags.len());
    for tag in &tags {
        println!("  - {tag}");
    }
    match &profile.active_tag {
        Some(tag) => println!("active_tag: {tag}"),
        None => println!("active_tag: (none)"),
    }
    Ok(())
}

fn cmd_gen(file: &Path, mtu: u32, mode: SplitModeArg, rules: &[String]) -> Result<()> {
    let profile = read_profile(file)?;
    let split = split_config(mode, rules)?;
    let settings = BuildSettings {
        mtu,
        ..BuildSettings::default()
    };
    let config = build_config(&profile, &split, &settings).context("building sing-box config")?;
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

async fn cmd_run(file: &Path, mtu: u32, mode: SplitModeArg, rules: &[String], binary: Option<PathBuf>) -> Result<()> {
    let profile = read_profile(file)?;
    let split = split_config(mode, rules)?;
    let settings = BuildSettings {
        mtu,
        ..BuildSettings::default()
    };
    let config = build_config(&profile, &split, &settings).context("building sing-box config")?;

    let binary = match binary {
        Some(path) => path,
        None => locate_resources().context("locating sing-box binary")?.singbox,
    };

    let work_dir = std::env::temp_dir().join("wisp-cli-run");
    tokio::fs::create_dir_all(&work_dir)
        .await
        .with_context(|| format!("creating work dir {}", work_dir.display()))?;

    let engine = SingBoxProcess::new(binary, work_dir, settings.clash_port, settings.clash_secret.clone());
    engine.start(config).await.context("starting sing-box")?;
    println!("sing-box started; printing stats every 2s, press Ctrl-C to stop");

    loop {
        tokio::select! {
            ctrl_c = tokio::signal::ctrl_c() => {
                ctrl_c.context("waiting for ctrl-c")?;
                println!("stopping...");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                match engine.stats().await {
                    Ok(stats) => println!("{stats:?}"),
                    Err(err) => eprintln!("stats error: {err:#}"),
                }
            }
        }
    }

    engine.stop().await.context("stopping sing-box")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Command::Parse { file } => cmd_parse(&file),
        Command::Gen { file, mtu, mode, rules } => cmd_gen(&file, mtu, mode, &rules),
        Command::Run {
            file,
            mtu,
            mode,
            rules,
            binary,
        } => cmd_run(&file, mtu, mode, &rules, binary).await,
    }
}
