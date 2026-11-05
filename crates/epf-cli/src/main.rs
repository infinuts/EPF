use clap::{Args, Parser, Subcommand, ValueEnum};
use epf_core::{
    diagnostics, engine, service, Error, OpenRequest, Protocol, Result, RuleStatus, Store,
    UpnpRouter,
};
use std::{
    io::{self, IsTerminal, Write},
    net::Ipv4Addr,
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    name = "epf",
    version,
    about = "Easy Port Forwarding. Your ports, without the router menus.",
    after_help = "Opening a mapping does not verify internet access. Rules expire after one hour by default."
)]
struct Cli {
    #[arg(long, global = true, help = "Use a different local configuration file")]
    config: Option<PathBuf>,
    #[arg(long, global = true, help = "Print machine-readable JSON")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open a port after confirmation
    Open {
        #[command(flatten)]
        rule: RuleArgs,
        #[arg(short = 'y', long, help = "Explicitly approve opening this port")]
        yes: bool,
    },
    /// Close EPF-created rules on a port (or one rule by --id)
    Close {
        #[arg(required_unless_present = "id", conflicts_with = "id", value_parser = clap::value_parser!(u16).range(1..))]
        port: Option<u16>,
        #[arg(long)]
        id: Option<String>,
        #[arg(short, long, value_enum, default_value = "both")]
        protocol: CliProtocol,
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// List saved rules; --refresh checks their current router state
    List {
        #[arg(long)]
        refresh: bool,
    },
    /// Detect the local network and UPnP router (read-only)
    Status,
    /// Explain likely NAT restrictions (read-only)
    Diagnose {
        #[arg(long, help = "Allow a public IP lookup via https://api4.ipify.org")]
        public_ip: bool,
    },
    /// Save or manage reusable forwarding configurations without opening ports
    Profiles {
        #[command(subcommand)]
        command: ProfileCommand,
    },
}

#[derive(Subcommand)]
enum ProfileCommand {
    Save {
        #[command(flatten)]
        rule: RuleArgs,
    },
    List,
    Delete {
        id: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliProtocol {
    Tcp,
    Udp,
    Both,
}
impl From<CliProtocol> for Protocol {
    fn from(value: CliProtocol) -> Self {
        match value {
            CliProtocol::Tcp => Self::Tcp,
            CliProtocol::Udp => Self::Udp,
            CliProtocol::Both => Self::Both,
        }
    }
}

#[derive(Args)]
struct RuleArgs {
    #[arg(value_parser = clap::value_parser!(u16).range(1..))]
    port: u16,
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..))]
    internal_port: Option<u16>,
    #[arg(short, long, value_enum, default_value = "tcp")]
    protocol: CliProtocol,
    #[arg(long, help = "Destination IPv4 on the detected local subnet")]
    destination: Option<Ipv4Addr>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value = "3600", value_parser = clap::value_parser!(u32).range(60..=86400))]
    lease: u32,
}
impl From<RuleArgs> for OpenRequest {
    fn from(value: RuleArgs) -> Self {
        Self {
            name: value.name.unwrap_or_else(|| format!("Port {}", value.port)),
            external_port: value.port,
            internal_port: value.internal_port.unwrap_or(value.port),
            protocol: value.protocol.into(),
            destination_ip: value.destination,
            lease_seconds: value.lease,
        }
    }
}

fn confirm(yes: bool, message: &str) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(Error::ConfirmationRequired);
    }
    eprint!("{message}\nType yes to continue: ");
    io::stderr()
        .flush()
        .map_err(|e| Error::Validation(e.to_string()))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| Error::Validation(e.to_string()))?;
    if input.trim() == "yes" {
        Ok(())
    } else {
        Err(Error::ConfirmationRequired)
    }
}

fn run(cli: Cli) -> Result<serde_json::Value> {
    let path = cli.config.map(Ok).unwrap_or_else(Store::default_path)?;
    match cli.command {
        Command::Open { rule, yes } => {
            let request = OpenRequest::from(rule);
            request.validate()?;
            confirm(yes, &format!("Open {} port {} → {}:{} for {} seconds? This allows incoming connections to that service.", request.protocol, request.external_port,
                request.destination_ip.map(|ip| ip.to_string()).unwrap_or("this computer".into()), request.internal_port, request.lease_seconds))?;
            let rules = service::open(&path, request, true)?;
            if !cli.json {
                println!(
                    "Router confirmed {} mapping(s). Internet accessibility has not been tested.",
                    rules.len()
                );
            }
            Ok(serde_json::json!({ "rules": rules, "reachability": "unverified" }))
        }
        Command::Close {
            port,
            id,
            protocol,
            yes,
        } => {
            let config = Store::open(&path)?.load()?;
            let protocols = Protocol::from(protocol).expand();
            let selected: Vec<_> = config
                .rules
                .iter()
                .filter(|r| {
                    if let Some(id) = &id {
                        r.id == *id
                    } else {
                        Some(r.external_port) == port
                            && protocols.contains(&r.protocol)
                            && r.status != RuleStatus::Inactive
                    }
                })
                .collect();
            if selected.is_empty() {
                return Err(Error::Validation(
                    "No matching EPF rule was found. Run epf list to see saved rules.".into(),
                ));
            }
            confirm(yes, &format!("Close {} EPF mapping(s)?", selected.len()))?;
            // Each close rechecks ownership. Stop and report partial completion on any failure.
            let mut closed = Vec::new();
            for rule in selected {
                service::close(&path, &rule.id, true).map_err(|e| {
                    Error::Partial(format!(
                        "Closed {} rule(s) before stopping: {e}",
                        closed.len()
                    ))
                })?;
                closed.push(rule.id.clone());
            }
            if !cli.json {
                println!("Closed {} mapping(s).", closed.len());
            }
            Ok(serde_json::json!({ "closed": closed }))
        }
        Command::List { refresh } => {
            let mut rules = if refresh {
                service::snapshot(&path)?.rules
            } else {
                Store::open(&path)?.load()?.rules
            };
            if !refresh {
                for rule in &mut rules {
                    rule.status = RuleStatus::Unknown;
                }
            }
            if !cli.json {
                if rules.is_empty() {
                    println!("No EPF rules yet. Try: epf open 25565");
                } else {
                    println!(
                        "{:<7} {:<6} {:<23} {:<10} NAME / ID",
                        "PORT", "PROTO", "DESTINATION", "STATUS"
                    );
                    for rule in &rules {
                        println!(
                            "{:<7} {:<6} {:<23} {:<10} {} / {}",
                            rule.external_port,
                            rule.protocol.to_string(),
                            format!("{}:{}", rule.destination_ip, rule.internal_port),
                            format!("{:?}", rule.status).to_lowercase(),
                            rule.name,
                            rule.id
                        );
                    }
                }
                if !refresh {
                    println!(
                        "Saved configuration only. Use --refresh to check current router state."
                    );
                }
            }
            Ok(serde_json::json!({ "rules": rules, "refreshed": refresh }))
        }
        Command::Status => {
            let (network, _) = UpnpRouter::discover();
            if !cli.json {
                println!(
                    "{}\nLocal IP: {}\nGateway: {}\nRouter WAN IP: {}\nUPnP: {}",
                    network.message,
                    display(network.local_ip),
                    display(network.gateway_ip),
                    display(network.wan_ip),
                    network.upnp_available
                );
            }
            Ok(serde_json::json!(network))
        }
        Command::Diagnose { public_ip } => {
            let (network, _) = UpnpRouter::discover();
            let report = diagnostics::run(network, public_ip);
            if !cli.json {
                println!(
                    "Local IP: {}\nRouter WAN IP: {}\nPublic IP: {}",
                    display(report.network.local_ip),
                    display(report.network.wan_ip),
                    display(report.public_ip)
                );
                for finding in &report.findings {
                    println!("• {finding}");
                }
                println!("{}", report.reachability);
            }
            Ok(serde_json::json!(report))
        }
        Command::Profiles { command } => {
            let store = Store::open(&path)?;
            match command {
                ProfileCommand::Save { rule } => {
                    let profile = engine::save_profile(&store, rule.into())?;
                    if !cli.json {
                        println!("Saved profile {}. No ports were opened.", profile.id);
                    }
                    Ok(serde_json::json!(profile))
                }
                ProfileCommand::List => {
                    let profiles = store.load()?.profiles;
                    if !cli.json {
                        for p in &profiles {
                            println!(
                                "{}  {}  {} / {}",
                                p.id, p.request.name, p.request.external_port, p.request.protocol
                            );
                        }
                    }
                    Ok(serde_json::json!({ "profiles": profiles }))
                }
                ProfileCommand::Delete { id } => {
                    engine::delete_profile(&store, &id)?;
                    if !cli.json {
                        println!("Deleted saved profile. Router rules were not changed.");
                    }
                    Ok(serde_json::json!({ "deleted": id }))
                }
            }
        }
    }
}

fn display(value: Option<impl std::fmt::Display>) -> String {
    value.map(|v| v.to_string()).unwrap_or("unavailable".into())
}

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    match run(cli) {
        Ok(value) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            }
        }
        Err(e) => {
            if json {
                eprintln!("{}", serde_json::json!({ "error": e.to_string() }));
            } else {
                eprintln!("EPF: {e}");
            }
            std::process::exit(
                if matches!(e, Error::Validation(_) | Error::ConfirmationRequired) {
                    2
                } else {
                    1
                },
            );
        }
    }
}
