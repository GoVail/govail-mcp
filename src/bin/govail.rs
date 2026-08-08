use std::env;
#[cfg(unix)]
use std::process::Command;
use std::process::ExitCode;

use govail_mcp::developer::DeliveryService;
use serde::Serialize;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.first().map(String::as_str) != Some("developer") {
        return Err(usage().into());
    }
    let command = args.get(1).map(String::as_str).ok_or_else(usage)?;
    let tail = &args[2..];
    let option_scope = if command == "verify" {
        let separator = tail
            .iter()
            .position(|value| value == "--")
            .ok_or("verify requires '-- <program> [args...]' after its options")?;
        &tail[..separator]
    } else {
        tail
    };
    let repo = option(option_scope, "--repo").unwrap_or_else(|| ".".into());
    let service = DeliveryService::open(repo)?;

    match command {
        "inspect" => print_json(&service.inspect()?)?,
        "commit" => {
            let message = required_option(tail, "--message")?;
            let files = repeated_option(tail, "--file");
            print_json(&serde_json::json!({
                "commit": service.commit_allowlisted(&files, &message)?
            }))?;
        }
        "verify" => {
            let name = required_option(option_scope, "--name")?;
            let separator = tail
                .iter()
                .position(|value| value == "--")
                .ok_or("verify requires '-- <program> [args...]' after its options")?;
            let program = tail
                .get(separator + 1)
                .ok_or("verification program is required")?;
            let program_args = tail[separator + 2..].to_vec();
            print_json(&service.verify(&name, program, &program_args)?)?;
        }
        "propose-push" => {
            let remote = option(tail, "--remote").unwrap_or_else(|| "origin".into());
            let branch = required_option(tail, "--branch")?;
            let verification = required_option(tail, "--verification")?;
            print_json(&service.propose_push(&remote, &branch, &verification)?)?;
        }
        "approve" => {
            if !tail.iter().any(|value| value == "--yes") {
                return Err("approve requires the explicit --yes flag".into());
            }
            let proposal = required_option(tail, "--proposal")?;
            let ttl = option(tail, "--ttl-seconds")
                .unwrap_or_else(|| "600".into())
                .parse()?;
            print_json(&service.approve(&proposal, &local_identity(), ttl)?)?;
        }
        "push" => {
            let approval = required_option(tail, "--approval")?;
            print_json(&service.push(&approval)?)?;
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|value| value == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn required_option(args: &[String], name: &str) -> Result<String, String> {
    option(args, name).ok_or_else(|| format!("missing required option {name}"))
}

fn repeated_option(args: &[String], name: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter(|(_, value)| *value == name)
        .filter_map(|(index, _)| args.get(index + 1).cloned())
        .collect()
}

fn print_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn local_identity() -> String {
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("id").arg("-un").output() {
            if output.status.success() {
                let user = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !user.is_empty() {
                    return format!("local:{user}");
                }
            }
        }
    }
    let user = env::var("USERNAME")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "unknown".into());
    format!("local:{user}")
}

fn usage() -> String {
    "usage: govail developer <inspect|commit|verify|propose-push|approve|push> [options]".into()
}
